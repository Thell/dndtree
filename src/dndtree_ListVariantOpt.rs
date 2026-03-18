use std::vec;

use nohash_hasher::{IntMap, IntSet};
use smallvec::SmallVec;

const MAX_DEPTH: i32 = 32767;

// MARK: LinkNode

#[derive(Clone, Copy, Debug)]
struct Link {
    prev: Option<usize>, // index in self.link_nodes
    next: Option<usize>,
}

impl Link {
    fn new() -> Self {
        Link {
            prev: None,
            next: None,
        }
    }
}

// MARK: Node

#[derive(Clone, Debug)]
struct Node {
    /// The adjacent neighbors of this node
    neighbors: SmallVec<[i32; 8]>,

    /// The parent of this node in the id-tree
    pub parent: i32,

    /// Subtree cardinality in normal operation. During rotations this field is
    /// temporarily used to store signed size deltas (child_size - parent_size)
    /// as part of the O(height) subtree-size transfer algorithm. The value is
    /// guaranteed to be >= 1 except while a rotation is actively in progress.
    pub subtree_size: i32,

    /// The root of the subtree to which this node belongs in the disjoint set
    pub root: i32,
}

impl Node {
    fn new() -> Self {
        Node {
            neighbors: SmallVec::new(),
            parent: -1,
            subtree_size: 0,
            root: -1,
        }
    }

    fn insert_neighbor(&mut self, u: i32) -> i32 {
        if !self.neighbors.contains(&u) {
            self.neighbors.push(u);
            return 0;
        }
        1
    }

    fn delete_neighbor(&mut self, u: i32) -> i32 {
        if let Some(i) = self.neighbors.iter().position(|&x| x == u) {
            self.neighbors.swap_remove(i);
            return 0;
        }
        1
    }
}

// MARK: DNDTree

/// DNDTree
#[derive(Clone, Debug)]
pub struct DNDTree {
    n: usize,
    nodes: Vec<Node>,

    l_nodes: Vec<Link>,            // big flat vec, size == n, index == node id
    root_head: Vec<Option<usize>>, // for each possible root: index of first real child in its list
    root_tail: Vec<Option<usize>>, // last real child (optional, helps splicing sometimes)

    used: Vec<bool>,
    vec_scratch_nodes: Vec<i32>,
    vec_scratch_stack: Vec<usize>,
    generation: u16,
    node_gen: Vec<u16>,

    use_union_find: bool,
    compress_links: bool,
}

impl DNDTree {
    /// Create a new DNDTree
    pub fn new(
        adj_dict: &IntMap<i32, IntSet<i32>>,
        use_union_find: bool,
        compress_links: bool,
    ) -> Self {
        let mut instance = Self::setup(&adj_dict, use_union_find, compress_links);
        instance.initialize();
        instance
    }

    /// Insert an undirected edge
    pub fn insert_edge(&mut self, u: usize, v: usize) -> i32 {
        if !self.insert_edge_in_graph(u, v) {
            return -1;
        }
        self.insert_edge_balanced(u, v)
    }

    /// Delete an undirected edge
    pub fn delete_edge(&mut self, u: usize, v: usize) -> i32 {
        if !self.delete_edge_in_graph(u, v) {
            return -1;
        }
        self.delete_edge_balanced(u, v)
    }

    /// Query if u and v are in the same connected component
    pub fn query(&mut self, u: usize, v: usize) -> bool {
        if u >= self.n || v >= self.n {
            return false;
        }

        if self.use_union_find {
            return self.get_dsu_root(u) == self.get_dsu_root(v);
        }

        let mut root_u = u;
        while self.nodes[root_u].parent != -1 {
            root_u = self.nodes[root_u].parent as usize;
        }

        let mut root_v = v;
        while self.nodes[root_v].parent != -1 {
            root_v = self.nodes[root_v].parent as usize;
        }

        root_u == root_v
    }
}

impl DNDTree {
    #[inline(always)]
    fn setup(
        adj_dict: &IntMap<i32, IntSet<i32>>,
        use_union_find: bool,
        compress_links: bool,
    ) -> Self {
        let n = adj_dict.len();
        let nodes: Vec<Node> = (0..n)
            .map(|i| {
                let mut node = Node::new();
                for &j in adj_dict.get(&(i as i32)).unwrap_or(&IntSet::default()) {
                    node.insert_neighbor(j);
                }
                node
            })
            .collect();

        Self {
            n,
            nodes,
            l_nodes: vec![],
            root_head: vec![],
            root_tail: vec![],
            used: vec![],
            vec_scratch_nodes: vec![],
            vec_scratch_stack: vec![],
            generation: 1,
            node_gen: vec![0; n],
            use_union_find: use_union_find,
            compress_links: compress_links,
        }
    }

    #[inline(always)]
    fn initialize(&mut self) {
        let n = self.n;
        let use_union_find = self.use_union_find;

        self.used = vec![false; n];

        let s = self.sort_nodes_by_degree();

        for v in 0..n {
            self.nodes[v].parent = -1;
            self.nodes[v].subtree_size = 1;
        }

        if use_union_find {
            self.l_nodes = (0..n).map(|_| Link::new()).collect();
            self.root_head = vec![None; n];
            self.root_tail = vec![None; n];

            for v in 0..n {
                self.nodes[v].root = v as i32;
                self.root_head[v] = Some(v);
                self.root_tail[v] = Some(v);
                self.l_nodes[v].prev = None;
                self.l_nodes[v].next = None;
            }
        }

        for &f in s.iter() {
            if self.used[f] {
                continue;
            }

            self.bfs_setup_subtrees(f, use_union_find);

            if let Some(centroid) = self.find_centroid_in_q() {
                self.reroot(centroid as usize, f as i32);
            }
        }

        self.used.fill(false);
    }

    #[inline(always)]
    fn sort_nodes_by_degree(&self) -> Vec<usize> {
        let mut node_indices: Vec<usize> = (0..self.n).collect();
        node_indices.sort_unstable_by(|&a, &b| {
            self.nodes[b]
                .neighbors
                .len()
                .cmp(&self.nodes[a].neighbors.len())
        });
        node_indices
    }

    #[inline(always)]
    fn bfs_setup_subtrees(&mut self, root: usize, use_union_find: bool) {
        use std::collections::VecDeque;
        let mut deque = VecDeque::new();
        deque.push_back(root as i32);

        self.vec_scratch_nodes.clear();
        self.vec_scratch_nodes.push(root as i32);

        self.used[root] = true;

        if use_union_find {
            self.splice_list(root, root);
            self.nodes[root].root = root as i32;
        }

        while let Some(p) = deque.pop_front() {
            for j in 0..self.nodes[p as usize].neighbors.len() {
                let nbr = self.nodes[p as usize].neighbors[j];
                let v = nbr as usize;
                if !self.used[v] {
                    self.used[v] = true;

                    self.nodes[v].parent = p;
                    self.vec_scratch_nodes.push(v as i32);
                    deque.push_back(v as i32);

                    if use_union_find {
                        self.nodes[v].root = root as i32;
                        self.splice_list(root, v);
                    }
                }
            }
        }

        for &q in self.vec_scratch_nodes.iter().skip(1).rev() {
            let p = self.nodes[q as usize].parent as usize;
            self.nodes[p].subtree_size += self.nodes[q as usize].subtree_size;
        }
    }

    #[inline(always)]
    fn find_centroid_in_q(&self) -> Option<i32> {
        let num_nodes = self.vec_scratch_nodes.len();
        let half_num_nodes = (num_nodes / 2) as i32;

        self.vec_scratch_nodes.iter().rev().find_map(|&node_index| {
            if self.nodes[node_index as usize].subtree_size > half_num_nodes {
                Some(node_index)
            } else {
                None
            }
        })
    }

    #[inline(always)]
    fn insert_edge_in_graph(&mut self, u: usize, v: usize) -> bool {
        if u >= self.n || v >= self.n || u == v {
            return false;
        }
        let inserted_u = self.nodes[u].insert_neighbor(v as i32);
        let inserted_v = self.nodes[v].insert_neighbor(u as i32);
        inserted_u == 0 && inserted_v == 0
    }

    #[inline(always)]
    fn insert_edge_balanced(&mut self, u: usize, v: usize) -> i32 {
        let (fu, fv): (usize, usize) = if !self.use_union_find {
            (self.get_tree_root(u), self.get_tree_root(v))
        } else {
            (self.get_dsu_root(u), self.get_dsu_root(v))
        };

        if fu == fv {
            self.insert_non_tree_edge_balanced(u, v, fu)
        } else {
            self.insert_tree_edge_balanced(u, v, fu, fv)
        }
    }

    /// Handles insertion of a non‑tree edge (u, v) when both endpoints are in the
    /// same component. This performs the depth‑imbalance check, identifies the
    /// centroid of the deeper side, detaches and reroots the smaller subtree, and
    /// rebalances the component around the centroid if required.
    ///
    /// Arguments:
    /// - `u`, `v`: original edge endpoints
    /// - `f`: the component root (tree‑root or DSU‑root), used to compute the
    ///        target half‑subtree size during rebalancing
    #[inline(always)]
    fn insert_non_tree_edge_balanced(&mut self, u: usize, v: usize, f: usize) -> i32 {
        let (reshape, small_node, large_node, small_p, _large_p) =
            self.detect_depth_imbalance(u, v);

        if !reshape {
            return 0;
        }

        // Node at which the subtree should be detached and rerooted.
        let p = self.find_imbalance_centroid(small_node, small_p) as usize;

        // Remove the subtree rooted at the detach point from its ancestors.
        self.adjust_subtree_sizes(p, -self.nodes[p].subtree_size);

        // Reroot the smaller subtree under the larger side.
        self.nodes[p].parent = -1;
        self.reroot(small_node, -1);
        self.nodes[small_node].parent = large_node as i32;

        // Recompute subtree sizes upward from the attach point and detect the new root centroid.
        let new_root = self.rebalance_tree(small_node, large_node, f);

        if new_root.is_some() && new_root != Some(f) {
            self.reroot(new_root.unwrap(), f as i32);
        }

        0
    }

    /// Determines whether the paths from u and v to the root differ enough to
    /// require a reshape. Walks both parent chains upward in lockstep until one
    /// reaches the root. If the other still has depth remaining, a reshape is
    /// required.
    ///
    /// Returns:
    /// - `reshape`: whether a rebalance is needed
    /// - `small_node`: the side that reached the root first (after swap)
    /// - `large_node`: the deeper side (after swap)
    /// - `small_p`: parent pointer at the divergence point for the shallow side
    /// - `large_p`: parent pointer at the divergence point for the deep side
    #[inline(always)]
    fn detect_depth_imbalance(&self, mut u: usize, mut v: usize) -> (bool, usize, usize, i32, i32) {
        let mut reshape = false;
        let mut depth = 0;

        let mut pu = self.nodes[u].parent;
        let mut pv = self.nodes[v].parent;

        while depth < MAX_DEPTH {
            if pu == -1 {
                if pv != -1 && self.nodes[pv as usize].parent != -1 {
                    reshape = true;
                    std::mem::swap(&mut u, &mut v);
                    std::mem::swap(&mut pu, &mut pv);
                }
                break;
            } else if pv == -1 {
                if pu != -1 && self.nodes[pu as usize].parent != -1 {
                    reshape = true;
                }
                break;
            }

            pu = self.nodes[pu as usize].parent;
            pv = self.nodes[pv as usize].parent;
            depth += 1;
        }

        (reshape, u, v, pu, pv)
    }

    /// Given the shallow side (`small_node`) and the parent pointer at the
    /// divergence point (`small_p`), computes the centroid of the deeper side.
    /// This is done by measuring the remaining depth to the root and walking
    /// halfway up.
    ///
    /// Arguments:
    /// - `small_node`: the node on the shallow side
    /// - `small_p`: parent pointer where the shallow side stopped
    ///
    /// Returns:
    /// - the centroid node index
    #[inline(always)]
    fn find_imbalance_centroid(&self, small_node: usize, small_p: i32) -> i32 {
        let mut depth_imbalance = 0;
        let mut p = small_p;

        while p != -1 {
            depth_imbalance += 1;
            p = self.nodes[p as usize].parent;
        }

        depth_imbalance = depth_imbalance / 2 - 1;
        let mut cur = small_node as i32;
        while depth_imbalance > 0 {
            cur = self.nodes[cur as usize].parent;
            depth_imbalance -= 1;
        }

        cur
    }

    /// Applies a constant subtree‑size adjustment to all ancestors of `start_node`.
    /// Used both for subtracting the detached subtree and for adding the attached
    /// subtree during rebalancing.
    ///
    /// Arguments:
    /// - `start_node`: the node whose subtree size is being propagated upward
    /// - `delta`: signed adjustment applied to each ancestor’s subtree_size
    ///
    /// Returns:
    /// - the last node whose subtree size was adjusted (the root)
    #[inline(always)]
    fn adjust_subtree_sizes(&mut self, start_node: usize, delta: i32) -> i32 {
        let mut cur = start_node;
        let mut p = self.nodes[start_node].parent;
        while p != -1 {
            self.nodes[p as usize].subtree_size += delta;
            cur = p as usize;
            p = self.nodes[p as usize].parent;
        }

        cur as i32
    }

    /// After attaching subtree `u` under node `v`, this propagates the subtree size
    /// of `u` upward through the ancestors of `v` and identifies the centroid of
    /// the merged component.
    ///
    /// Arguments:
    /// - `u`: root of the newly attached subtree
    /// - `v`: attach point in the larger component
    /// - `f`: root of the larger component (used to compute the half‑size threshold)
    ///
    /// Returns:
    /// - `Some(new_root)` if a centroid different from `f` is found
    /// - `None` if no rebalance is needed
    #[inline(always)]
    fn rebalance_tree(&mut self, u: usize, v: usize, f: usize) -> Option<usize> {
        let s = (self.nodes[f].subtree_size + self.nodes[u].subtree_size) / 2;
        let mut new_root = None;
        let mut p = v as i32;
        while p != -1 {
            self.nodes[p as usize].subtree_size += self.nodes[u].subtree_size;
            if new_root.is_none() && self.nodes[p as usize].subtree_size > s {
                new_root = Some(p as usize);
            }
            p = self.nodes[p as usize].parent;
        }
        new_root
    }

    /// Handles insertion of a tree edge (u, v) connecting two different components.
    /// Ensures the smaller component attaches under the larger one, rotating the
    /// tree so that `u` becomes the root of its component, fixes subtree sizes
    /// along the reversed path, and rebalances the merged tree.
    ///
    /// Arguments:
    /// - `u`, `v`: edge endpoints
    /// - `fu`: root of u’s component
    /// - `fv`: root of v’s component
    #[inline(always)]
    fn insert_tree_edge_balanced(&mut self, u: usize, v: usize, fu: usize, fv: usize) -> i32 {
        let mut u = u;
        let mut v = v;
        let mut fu = fu;
        let mut fv = fv;

        // Attach smaller component under larger.
        if self.nodes[fu].subtree_size > self.nodes[fv].subtree_size {
            std::mem::swap(&mut u, &mut v);
            std::mem::swap(&mut fu, &mut fv);
        }

        self.rotate_tree(u, v);
        self.fix_rotated_subtree_sizes(u, v);

        let new_root = self.rebalance_tree(fu, v, fv);

        if self.use_union_find {
            self.union_f(fu, fv);
        }

        if new_root.is_some() && new_root != Some(fv) {
            self.reroot(new_root.unwrap(), fv as i32);
        }

        1
    }

    /// Rotates the parent pointers along the branch from `start_node` upward so that
    /// `start_node` becomes the root of that branch, then attaches the branch under
    /// `stop_node`.
    ///
    /// Arguments:
    /// - `start_node`: node whose branch is being rotated
    /// - `stop_node`: attach point in the other component
    #[inline(always)]
    fn rotate_tree(&mut self, start_node: usize, stop_node: usize) {
        self._rotate_tree(start_node, stop_node as i32);
    }

    /// After a rotation updates the parent chain of a component, this restores
    /// correct subtree sizes along the affected branch until reaching `stop_node`.
    ///
    /// Arguments:
    /// - `start_node`: the node where the updated branch begins
    /// - `stop_node`: the node at which to stop adjusting (the attach point)
    #[inline(always)]
    fn fix_rotated_subtree_sizes(&mut self, start_node: usize, stop_node: usize) {
        self._fix_rotated_subtree_sizes(start_node, stop_node as i32);
    }

    #[inline(always)]
    fn delete_edge_in_graph(&mut self, u: usize, v: usize) -> bool {
        if u >= self.n || v >= self.n || u == v {
            return false;
        }
        let deleted_u = self.nodes[u].delete_neighbor(v as i32);
        let deleted_v = self.nodes[v].delete_neighbor(u as i32);
        deleted_u == 0 && deleted_v == 0
    }

    #[inline(always)]
    fn delete_edge_balanced(&mut self, mut u: usize, mut v: usize) -> i32 {
        if (self.nodes[u].parent != v as i32 && self.nodes[v].parent != u as i32) || u == v {
            return 0;
        }

        if self.nodes[v].parent == u as i32 {
            std::mem::swap(&mut u, &mut v);
        }

        let p = self.adjust_subtree_sizes(v, -self.nodes[u].subtree_size) as usize;
        self.nodes[u].parent = -1;

        let (small_node, large_node): (usize, usize) =
            if self.nodes[u].subtree_size < self.nodes[p].subtree_size {
                (u, p)
            } else {
                if self.use_union_find {
                    self.nodes[p].root = u as i32;
                    self.splice_list(p, u);
                    self.nodes[u].root = u as i32;
                }

                (p, u)
            };

        if self.find_replacement(small_node, large_node) {
            return 1;
        }

        if self.use_union_find {
            self.remove_subtree_union_find(small_node);
        }

        2
    }

    /// Searches for a non‑tree edge that still connects the two components
    /// created by deleting a tree edge. If such an edge exists, the function
    /// rebuilds the ID‑Tree structure around that edge so that the component
    /// remains a single balanced tree.
    ///
    /// From the IDTree and DSU perspective, the component is already either
    /// connected or disconnected; this function does not determine that fact.
    /// It only determines whether a valid replacement edge exists and, if so,
    /// performs the structural rotations and rebalancing needed to make that
    /// edge the new tree connection.
    ///
    /// Returns `true` if a replacement edge was found and the tree structure
    /// was rebuilt around it; otherwise returns `false`, leaving the two
    /// components permanently separated.
    ///
    /// Arguments:
    /// - `u`: the root of the detached subtree
    /// - `f`: the root of the other component
    ///
    /// Returns:
    /// - `true` if a replacement edge was found and the tree structure was rebuilt
    #[inline(always)]
    fn find_replacement(&mut self, u: usize, f: usize) -> bool {
        self.vec_scratch_nodes.clear();
        self.vec_scratch_stack.clear();

        self.vec_scratch_nodes.push(u as i32);
        self.vec_scratch_stack.push(u);

        self.generation = self.generation.wrapping_add(1);
        let cur_gen = self.generation;

        self.node_gen[u] = cur_gen;

        let mut i = 0;
        while i < self.vec_scratch_nodes.len() {
            let node = self.vec_scratch_nodes[i];
            i += 1;

            let mut j = 0;
            while j < self.nodes[node as usize].neighbors.len() {
                let neighbor = self.nodes[node as usize].neighbors[j];
                if neighbor == self.nodes[node as usize].parent {
                    j += 1;
                    continue;
                }

                if self.nodes[neighbor as usize].parent == node as i32 {
                    self.vec_scratch_nodes.push(neighbor);
                    if self.node_gen[neighbor as usize] != cur_gen {
                        self.node_gen[neighbor as usize] = cur_gen;
                        self.vec_scratch_stack.push(neighbor as usize);
                    }
                    j += 1;
                    continue;
                }

                // Check whether this neighbor’s parent chain reaches the other component
                // without intersecting the detached subtree, making it a valid replacement edge.
                let mut succ = true;
                let mut w = neighbor;
                while w != -1 {
                    if self.node_gen[w as usize] == cur_gen {
                        succ = false;
                        break;
                    }
                    self.node_gen[w as usize] = cur_gen;
                    self.vec_scratch_stack.push(w as usize);

                    w = self.nodes[w as usize].parent;
                }
                if !succ {
                    j += 1;
                    continue;
                }

                self.rotate_tree(node as usize, neighbor as usize);
                self.fix_rotated_subtree_sizes(node as usize, neighbor as usize);
                let new_root = self.rebalance_tree(u, neighbor as usize, f);

                if new_root.is_some() && new_root != Some(f) {
                    self.reroot(new_root.unwrap(), f as i32);
                }

                return true;
            }
        }

        false
    }

    #[inline(always)]
    // fn get_tree_root(&mut self, u: usize) -> usize {
    //     let mut root = u;
    //     while self.nodes[root].parent != -1 {
    //         root = self.nodes[root].parent as usize;
    //     }

    //     let mut cur = u;
    //     while cur != root {
    //         let p = self.nodes[cur].parent as usize;
    //         if p != root {
    //             self.nodes[cur].parent = self.nodes[p].parent;
    //         }
    //         cur = p;
    //     }

    //     root
    // }
    fn get_tree_root(&mut self, u: usize) -> usize {
        let mut root = u;
        while self.nodes[root].parent != -1 {
            root = self.nodes[root].parent as usize;
        }
        root
    }

    #[inline(always)]
    fn get_dsu_root(&mut self, u: usize) -> usize {
        // Phase 1: find true root (read-only, same as before)
        let mut root = u;
        while self.nodes[root].root as usize != root {
            root = self.nodes[root].root as usize;
        }

        if self.compress_links {
            // Strong mode: full flattening + relocate every node to root's list
            let mut cur = u;
            while self.nodes[cur].root as usize != root {
                let next = self.nodes[cur].root as usize;

                self.isolate_link(cur);
                self.insert_link_to_root(root, cur);

                self.nodes[cur].root = root as i32;
                cur = next;
            }
        } else {
            // Weak mode: apply halving (grandparent) instead of direct-to-root
            let mut cur = u;
            while self.nodes[cur].root as usize != root {
                let next = self.nodes[cur].root as usize;
                let grandparent = self.nodes[next].root;
                self.nodes[cur].root = grandparent;
                cur = next;
            }
        }

        root
    }

    #[inline(always)]
    fn reroot(&mut self, u: usize, f: i32) {
        let old_root = self.rotate_tree_to_root(u);
        self.fix_rotated_subtree_sizes_until_root(old_root);

        if self.use_union_find && f >= 0 {
            self.nodes[f as usize].root = u as i32;
            self.splice_list(f as usize, u);
            self.nodes[u].root = u as i32;
        }
    }

    /// Rotates the parent pointers along the branch from `start_node` upward so that
    /// `start_node` becomes the root of that branch, then attaches the branch under
    /// `new_parent`.
    ///
    /// Arguments:
    /// - `start_node`: node whose branch is being rotated
    /// - `new_parent`: the parent value to attach the rotated branch under
    #[inline(always)]
    fn _rotate_tree(&mut self, mut u: usize, new_parent: i32) -> usize {
        let mut p = self.nodes[u].parent;
        self.nodes[u].parent = new_parent;

        while p != -1 {
            let next = self.nodes[p as usize].parent;
            self.nodes[p as usize].parent = u as i32;
            u = p as usize;
            p = next;
        }

        u // old root
    }

    /// After a rotation updates the parent chain of a component, this restores
    /// correct subtree sizes along the affected branch until reaching `stop_parent`.
    ///
    /// Arguments:
    /// - `start_node`: the node where the updated branch begins
    /// - `stop_parent`: the parent value at which to stop adjusting
    #[inline(always)]
    fn _fix_rotated_subtree_sizes(&mut self, mut u: usize, stop_parent: i32) {
        let mut p = self.nodes[u].parent;

        while p != stop_parent {
            let parent_idx = p as usize;

            self.nodes[u].subtree_size -= self.nodes[parent_idx].subtree_size;
            self.nodes[parent_idx].subtree_size += self.nodes[u].subtree_size;

            u = parent_idx;
            p = self.nodes[u].parent;
        }
    }

    /// Rotates the parent pointers along the branch from `start_node` to the root,
    /// so that `start_node` becomes the root of its component.
    ///
    /// Arguments:
    /// - `start_node`: node whose component is being rerooted
    #[inline(always)]
    fn rotate_tree_to_root(&mut self, start_node: usize) -> usize {
        self._rotate_tree(start_node, -1)
    }

    /// After a rotation updates the parent chain of a component, this restores
    /// correct subtree sizes along the affected branch until reaching the root.
    ///
    /// Arguments:
    /// - `start_node`: the node where the updated branch begins
    #[inline(always)]
    fn fix_rotated_subtree_sizes_until_root(&mut self, start_node: usize) {
        self._fix_rotated_subtree_sizes(start_node, -1);
    }

    #[inline(always)]
    fn remove_subtree_union_find(&mut self, u: usize) {
        let detached_root = u;
        self.nodes[u].root = u as i32;

        for &v in &self.vec_scratch_nodes {
            self.nodes[v as usize].root = detached_root as i32;
        }
    }

    #[inline(always)]
    fn union_f(&mut self, fu: usize, fv: usize) {
        if fu == fv {
            return;
        }
        self.nodes[fu].root = fv as i32;
        self.splice_list(fu, fv);
    }

    #[inline(always)]
    fn isolate_link(&mut self, idx: usize) {
        let prev = self.l_nodes[idx].prev;
        let next = self.l_nodes[idx].next;

        if let Some(p) = prev {
            self.l_nodes[p].next = next;
        }
        if let Some(n) = next {
            self.l_nodes[n].prev = prev;
        }

        self.l_nodes[idx].prev = None;
        self.l_nodes[idx].next = None;
    }

    // Insert single node 'child_idx' into the list of root 'r'
    #[inline(always)]
    fn insert_link_to_root(&mut self, r: usize, child_idx: usize) {
        self.l_nodes[child_idx].prev = None;

        if let Some(old_head) = self.root_head[r] {
            self.l_nodes[child_idx].next = Some(old_head);
            self.l_nodes[old_head].prev = Some(child_idx);
            self.root_head[r] = Some(child_idx);
        } else {
            self.root_head[r] = Some(child_idx);
            self.root_tail[r] = Some(child_idx);
            self.l_nodes[child_idx].next = None;
        }
    }

    /// Bulk splice: move entire list from old_root to new_root (append or prepend)
    #[inline(always)]
    fn splice_list(&mut self, from_root: usize, to_root: usize) {
        let Some(head_from) = self.root_head[from_root] else {
            return;
        };
        let Some(tail_from) = self.root_tail[from_root] else {
            return;
        };

        self.root_head[from_root] = None;
        self.root_tail[from_root] = None;

        if let Some(head_to) = self.root_head[to_root] {
            self.l_nodes[tail_from].next = Some(head_to);
            self.l_nodes[head_to].prev = Some(tail_from);
            self.root_tail[to_root] = Some(tail_from);
        } else {
            self.root_head[to_root] = Some(head_from);
            self.root_tail[to_root] = Some(tail_from);
        }

        self.l_nodes[head_from].prev = None;
        self.l_nodes[tail_from].next = None;
    }
}

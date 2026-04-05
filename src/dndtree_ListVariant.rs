use std::vec;

use nohash_hasher::{IntMap, IntSet};
use smallvec::SmallVec;

const MAX_DEPTH: usize = 32767;
const SENTINEL: usize = usize::MAX;

// MARK: Link

#[derive(Clone, Copy, Debug)]
struct Link {
    prev: usize,
    next: usize,
}

impl Link {
    fn new() -> Self {
        Link {
            prev: SENTINEL,
            next: SENTINEL,
        }
    }
}

// MARK: Node

#[derive(Clone, Debug)]
pub struct Node {
    /// The parent of this node in the id-tree
    pub parent: usize,

    /// Subtree cardinality in normal operation. During rotations this field is
    /// temporarily used to store signed size deltas (child_size - parent_size)
    /// as part of the O(height) subtree-size transfer algorithm. The value is
    /// guaranteed to be >= 1 except while a rotation is actively in progress.
    pub subtree_size: i32,

    /// The adjacent neighbors of this node
    pub neighbors: SmallVec<[u32; 8]>,
}

impl Node {
    fn new() -> Self {
        Node {
            parent: SENTINEL,
            subtree_size: 1,
            neighbors: SmallVec::new(),
        }
    }

    fn insert_neighbor(&mut self, u: u32) -> i32 {
        if !self.neighbors.contains(&u) {
            self.neighbors.push(u);
            // // Sorting is for use during the development cycle for divergence testing of op logic
            // self.neighbors.sort();
            return 0;
        }
        1
    }

    fn delete_neighbor(&mut self, u: u32) -> i32 {
        if let Some(i) = self.neighbors.iter().position(|&x| x == u) {
            self.neighbors.swap_remove(i);
            // // Sorting is for use during the development cycle for divergence testing of op logic
            // self.neighbors.sort();
            return 0;
        }
        1
    }
}

/// MARK: DNDTree
//
// NOTE: After setup completes all node, neighbor and link entries are
//       guaranteed to be within range 0..self.n
// SAFETY: No function should be added to the struct that allows direct modification
//         of any of these fields and all public functions must check the invariants.
//         ( 0 <= u < self.n, 0 <= v < self.n, 0 <= u < self.n, 0 <= v < self.n )
#[derive(Clone, Debug)]
pub struct DNDTree {
    n: usize,
    use_union_find: bool,

    nodes: Vec<Node>,
    generation: u16,
    generations: Vec<u16>,
    vec_scratch_nodes: Vec<usize>,

    l_nodes: Vec<Link>,
    children_head: Vec<usize>,
    children_tail: Vec<usize>,
    link_parent: Vec<usize>,
    roots: Vec<usize>,
}

impl DNDTree {
    /// Create a new DNDTree
    pub fn new(adj_dict: &IntMap<i32, IntSet<i32>>, use_union_find: bool) -> Self {
        let mut instance = Self::setup(&adj_dict, use_union_find);
        instance.initialize();
        instance
    }

    /// Insert an undirected edge
    ///
    /// Returns:
    ///   -1 if the edge is invalid
    ///   0 if the edge inserted was a non-tree edge
    ///   1 if the edge inserted was a tree edge
    ///   2 if the edge inserted was a non-tree edge triggering a reroot
    ///   3 if the edge inserted was a tree edge triggering a reroot
    pub fn insert_edge(&mut self, u: usize, v: usize) -> i32 {
        if u >= self.n || v >= self.n || u == v || !self.insert_edge_in_graph(u, v) {
            return -1;
        }
        let res = self.insert_edge_balanced(u, v);
        res
    }

    /// Delete an undirected edge
    ///
    /// Returns:
    ///   -1 if the edge is invalid
    ///   0 if the edge deleted was a non-tree edge
    ///   1 if the edge deleted was a tree edge
    ///   2 if the edge deleted was a tree edge and a replacement edge was found
    pub fn delete_edge(&mut self, u: usize, v: usize) -> i32 {
        if u >= self.n || v >= self.n || u == v || !self.delete_edge_in_graph(u, v) {
            return -1;
        }
        let res = self.delete_edge_balanced(u, v);
        res
    }

    /// Query if u and v are in the same connected component
    //
    // NOTE: mut is required for DSU path and link compression
    pub fn query(&mut self, u: usize, v: usize) -> bool {
        if u >= self.n || v >= self.n {
            return false;
        }
        if self.use_union_find {
            return self.get_dsu_root(u) == self.get_dsu_root(v);
        }
        self.get_tree_root(u) == self.get_tree_root(v)
    }

    /// TODO: Remove after debugging
    pub fn get_node_data(&self, u: usize) -> Node {
        self.nodes[u].clone()
    }
}

impl DNDTree {
    // NOTE: After setup completes all node, neighbor and lnode entries are
    //       guaranteed to be within range 0..self.n
    // SAFETY: No function should be added to the struct that allows direct modification
    //         of any of these fields
    #[inline(always)]
    fn setup(adj_dict: &IntMap<i32, IntSet<i32>>, use_union_find: bool) -> Self {
        let n = adj_dict.len();
        let nodes: Vec<Node> = (0..n)
            .map(|i| {
                let mut node = Node::new();
                for &j in adj_dict.get(&(i as i32)).unwrap_or(&IntSet::default()) {
                    assert!(
                        j >= 0 && j < n as i32,
                        "invalid neighbor {} of {}",
                        j,
                        adj_dict.len()
                    );
                    node.insert_neighbor(j as u32);
                }
                node
            })
            .collect();

        Self {
            n,
            use_union_find,
            nodes,
            generation: 1,
            generations: vec![0; n],
            vec_scratch_nodes: Vec::with_capacity(n),
            l_nodes: Vec::with_capacity(n),
            children_head: Vec::with_capacity(n),
            children_tail: Vec::with_capacity(n),
            link_parent: Vec::with_capacity(n),
            roots: Vec::with_capacity(n),
        }
    }

    #[inline(always)]
    fn initialize(&mut self) {
        let use_union_find = self.use_union_find;

        let cur_generation = self.next_generation();

        let sorted_nodes = self.sort_nodes_by_degree();

        if use_union_find {
            self.init_dsu_lists();
        }

        for &node in sorted_nodes.iter() {
            // NOTE: generation is set for the subtree nodes in bfs_setup_subtrees
            if self.generations[node] == cur_generation {
                continue;
            }

            // NOTE: each subtree is setup in the scratch collection which is reused
            //       to find the centroid
            self.bfs_setup_subtrees(node, use_union_find);
            if let Some(centroid) = self.find_centroid_in_q() {
                self.reroot(centroid, node);
            }
        }

        self.vec_scratch_nodes.clear();
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
        deque.push_back(root);

        self.vec_scratch_nodes.clear();
        self.vec_scratch_nodes.push(root);

        let cur_generation = self.generation;
        self.generations[root] = cur_generation;

        if use_union_find {
            self.roots[root] = root;
            self.insert_child(root, root);
        }

        while let Some(p) = deque.pop_front() {
            for j in 0..self.nodes[p].neighbors.len() {
                let neighbor = self.nodes[p].neighbors[j] as usize;

                if self.generations[neighbor] != cur_generation {
                    self.generations[neighbor] = cur_generation;

                    self.nodes[neighbor].parent = p;
                    self.vec_scratch_nodes.push(neighbor);
                    deque.push_back(neighbor);

                    if use_union_find {
                        self.roots[neighbor] = root;
                        self.insert_child(root, neighbor);
                    }
                }
            }
        }

        for &q in self.vec_scratch_nodes.iter().skip(1).rev() {
            let p = self.nodes[q].parent;
            self.nodes[p].subtree_size += self.nodes[q].subtree_size;
        }
    }

    #[inline(always)]
    // NOTE: Uses pre-populated self.vec_scratch_nodes from bfs_setup_subtrees.
    fn find_centroid_in_q(&self) -> Option<usize> {
        let num_nodes = self.vec_scratch_nodes.len();
        let half_num_nodes = (num_nodes / 2) as i32;

        self.vec_scratch_nodes.iter().rev().find_map(|&i| {
            if self.nodes[i].subtree_size > half_num_nodes {
                Some(i)
            } else {
                None
            }
        })
    }
}

impl DNDTree {
    // MARK: Accessors
    // SAFETY: Unchecked access is safe because all public functions check invariants
    //         and after setup completes all entries are within range 0..self.n with
    //         proper invariants and all node accesses are within range 0..self.n.
    // NOTE: Sentinel value of usize::MAX is reserved for NULL for parent usage only
    // TODO: Switch to NonMax type once stable https://github.com/rust-lang/rust/issues/151435
    #[inline(always)]
    fn node(&self, i: usize) -> &Node {
        debug_assert!(i < self.n);
        unsafe { self.nodes.get_unchecked(i) }
    }

    #[inline(always)]
    fn root(&self, i: usize) -> usize {
        debug_assert!(i < self.n);
        unsafe { *self.roots.get_unchecked(i) }
    }

    #[inline(always)]
    fn root_mut(&mut self, i: usize) -> &mut usize {
        debug_assert!(i < self.n);
        unsafe { self.roots.get_unchecked_mut(i) }
    }

    #[inline(always)]
    fn next_generation(&mut self) -> u16 {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.generation = 1;
            self.generations.fill(0);
        }
        self.generation
    }
}

// MARK: Base functions

impl DNDTree {
    #[inline(always)]
    fn delete_edge_in_graph(&mut self, u: usize, v: usize) -> bool {
        self.nodes[u].delete_neighbor(v as u32) == 0 && self.nodes[v].delete_neighbor(u as u32) == 0
    }

    #[inline(always)]
    fn delete_edge_balanced(&mut self, mut u: usize, mut v: usize) -> i32 {
        if (self.nodes[u].parent != v && self.nodes[v].parent != u) || u == v {
            return 0;
        }

        if self.nodes[v].parent == u {
            std::mem::swap(&mut u, &mut v);
        }

        let (p, subtree_u_size) = self.unlink(u, v);
        let (small_node, large_node): (usize, usize) =
            if self.nodes[p].subtree_size < subtree_u_size {
                if self.use_union_find {
                    self.reroot_dsu(u, p);
                }
                (p, u)
            } else {
                (u, p)
            };

        // NOTE: Populates self.vec_scratch_nodes for potential re-use by remove_subtree_union_find
        if self.find_replacement(small_node, large_node) {
            return 1;
        }

        if self.use_union_find {
            self.remove_subtree_union_find(small_node, large_node);
        }
        2
    }

    #[inline(always)]
    fn insert_edge_in_graph(&mut self, u: usize, v: usize) -> bool {
        self.nodes[u].insert_neighbor(v as u32) == 0 && self.nodes[v].insert_neighbor(u as u32) == 0
    }

    #[inline(always)]
    fn insert_edge_balanced(&mut self, u: usize, v: usize) -> i32 {
        let (fu, fv) = if !self.use_union_find {
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
        let p = self.find_imbalance_centroid(small_node, small_p);

        // Remove the subtree rooted at the detach point from its ancestors.
        self.adjust_subtree_sizes(p, -self.nodes[p].subtree_size);

        // Reroot the smaller subtree under the larger side.
        self.nodes[p].parent = SENTINEL;
        self.reroot(small_node, SENTINEL);
        self.nodes[small_node].parent = large_node;

        // Recompute subtree sizes upward from the attach point and detect the new root centroid.
        let new_root = self.rebalance_tree(small_node, large_node, f);

        if let Some(new_root) = new_root
            && new_root != f
        {
            self.reroot(new_root, f);
            return 2;
        }

        0
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
    fn insert_tree_edge_balanced(
        &mut self,
        mut u: usize,
        mut v: usize,
        mut fu: usize,
        mut fv: usize,
    ) -> i32 {
        // Ensure fu is the root of the smaller component.
        if self.nodes[fu].subtree_size > self.nodes[fv].subtree_size {
            std::mem::swap(&mut u, &mut v);
            std::mem::swap(&mut fu, &mut fv);
        }

        let u = self.rotate_tree(u, v);

        // Attach smaller component under larger.
        let new_root = self.rebalance_tree(fu, v, fv);

        self.fix_rotated_subtree_sizes(u, v);

        if self.use_union_find {
            self.union_f(fu, fv);
        }

        if let Some(new_root) = new_root
            && new_root != fv
        {
            self.reroot(new_root, fv);
            return 3;
        }

        1
    }

    #[inline(always)]
    fn get_tree_root(&self, u: usize) -> usize {
        let mut root = u;
        while self.node(root).parent != SENTINEL {
            root = self.nodes[root].parent;
        }
        root
    }
}

// MARK: Support functions

impl DNDTree {
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
    fn detect_depth_imbalance(
        &self,
        mut u: usize,
        mut v: usize,
    ) -> (bool, usize, usize, usize, usize) {
        let mut reshape = false;
        let mut depth = 0;

        let mut pu = self.nodes[u].parent;
        let mut pv = self.nodes[v].parent;

        while depth < MAX_DEPTH {
            if pu == SENTINEL {
                if pv != SENTINEL && self.nodes[pv].parent != SENTINEL {
                    reshape = true;
                    std::mem::swap(&mut u, &mut v);
                    std::mem::swap(&mut pu, &mut pv);
                }
                break;
            } else if pv == SENTINEL {
                if pu != SENTINEL && self.nodes[pu].parent != SENTINEL {
                    reshape = true;
                }
                break;
            }

            pu = self.nodes[pu].parent;
            pv = self.nodes[pv].parent;
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
    fn find_imbalance_centroid(&self, small_node: usize, small_p: usize) -> usize {
        let mut depth_imbalance = 0;
        let mut p = small_p;

        while p != SENTINEL {
            depth_imbalance += 1;
            p = self.nodes[p].parent;
        }

        depth_imbalance = depth_imbalance / 2 - 1;

        let mut cur = small_node;
        while depth_imbalance > 0 {
            cur = self.nodes[cur].parent;
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
    fn adjust_subtree_sizes(&mut self, start_node: usize, delta: i32) -> usize {
        let mut root_v = start_node;
        let mut w = self.nodes[start_node].parent;
        while w != SENTINEL {
            self.nodes[w].subtree_size += delta;
            root_v = w;
            w = self.nodes[w].parent;
        }

        root_v
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
        let mut p = v;

        while p != SENTINEL {
            self.nodes[p].subtree_size += self.nodes[u].subtree_size;
            if new_root.is_none() && self.nodes[p].subtree_size > s {
                new_root = Some(p);
            }
            p = self.nodes[p].parent;
        }

        new_root
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
    /// - `root_v`: the root of the other component
    ///
    /// Returns:
    /// - `true` if a replacement edge was found and the tree structure was rebuilt
    #[inline(always)]
    fn find_replacement(&mut self, u: usize, root_v: usize) -> bool {
        self.vec_scratch_nodes.clear();
        let cur_generation = self.next_generation();

        self.vec_scratch_nodes.push(u);
        self.generations[u] = cur_generation;

        // NOTE: Do not use a deque here for the queue since popping from the front removes elements
        //       and when use_union_find is true the scratch vec is used as the subtree to
        //       to remove from the DSU via the remove subtree processing.
        let mut i = 0;
        while i < self.vec_scratch_nodes.len() {
            let node = self.vec_scratch_nodes[i];
            i += 1;

            'neighbors: for n_idx in 0..self.nodes[node].neighbors.len() {
                let neighbor = self.nodes[node].neighbors[n_idx] as usize;
                if neighbor == self.nodes[node].parent {
                    continue;
                }

                // NOTE: It is tempting to short-circuit this loop with
                //         `&& self.generations[neighbor] != cur_generation`
                //       but that can cause improper subtree setup in the scratch collection
                //       (See the with_dsu::test_mixed_ops_query_heavy test case.)
                //       For a non-DSU dedicated build for a specific graph this may be worth the
                //       performance optimization but requires careful analysis.
                if self.nodes[neighbor].parent == node {
                    self.vec_scratch_nodes.push(neighbor);
                    self.generations[neighbor] = cur_generation;
                    continue;
                }

                let mut w = neighbor;
                while w != SENTINEL {
                    if self.generations[w] == cur_generation {
                        continue 'neighbors;
                    }
                    w = self.nodes[w].parent;
                }

                let rotated_u = self.rotate_tree(node, neighbor);
                let new_root = self.rebalance_tree(rotated_u, neighbor, root_v);
                self.fix_rotated_subtree_sizes(rotated_u, neighbor);

                if let Some(new_root) = new_root
                    && new_root != root_v
                {
                    self.reroot(new_root, root_v);
                }
                return true;
            }
        }
        false
    }

    /// Reroots the tree by moving the subtree of `u` to `f`.
    #[inline(always)]
    fn reroot(&mut self, u: usize, f: usize) {
        let old_root = self.rotate_tree_to_root(u);
        self.fix_rotated_subtree_sizes_until_root(old_root);

        if self.use_union_find && f != SENTINEL {
            self.reroot_dsu(u, f);
        }
    }

    /// Rotates the parent pointers along the branch from `start_node` upward so that
    /// `start_node` becomes the root of that branch, then attaches the branch under
    /// `stop_node`.
    ///
    /// Arguments:
    /// - `start_node`: node whose branch is being rotated
    /// - `stop_node`: attach point in the other component
    #[inline(always)]
    fn rotate_tree(&mut self, start_node: usize, stop_node: usize) -> usize {
        self._rotate_tree(start_node, stop_node)
    }

    /// Rotates the parent pointers along the branch from `start_node` to the root,
    /// so that `start_node` becomes the root of its component.
    ///
    /// Arguments:
    /// - `start_node`: node whose component is being rerooted
    #[inline(always)]
    fn rotate_tree_to_root(&mut self, start_node: usize) -> usize {
        self._rotate_tree(start_node, SENTINEL)
    }

    /// Rotates the parent pointers along the branch from `start_node` upward so that
    /// `start_node` becomes the root of that branch, then attaches the branch under
    /// `new_parent`.
    ///
    /// Arguments:
    /// - `start_node`: node whose branch is being rotated
    /// - `new_parent`: the parent value to attach the rotated branch under
    #[inline(always)]
    fn _rotate_tree(&mut self, mut u: usize, new_parent: usize) -> usize {
        let mut p = self.nodes[u].parent;
        self.nodes[u].parent = new_parent;

        while p != SENTINEL {
            let next = self.nodes[p].parent;
            self.nodes[p].parent = u;
            u = p;
            p = next;
        }

        u // old root
    }

    /// After a rotation updates the parent chain of a component, this restores
    /// correct subtree sizes along the affected branch until reaching `stop_node`.
    ///
    /// Arguments:
    /// - `start_node`: the node where the updated branch begins
    /// - `stop_node`: the node at which to stop adjusting (the attach point)
    #[inline(always)]
    fn fix_rotated_subtree_sizes(&mut self, start_node: usize, stop_node: usize) {
        self._fix_rotated_subtree_sizes(start_node, stop_node);
    }

    /// After a rotation updates the parent chain of a component, this restores
    /// correct subtree sizes along the affected branch until reaching the root.
    ///
    /// Arguments:
    /// - `start_node`: the node where the updated branch begins
    #[inline(always)]
    fn fix_rotated_subtree_sizes_until_root(&mut self, start_node: usize) {
        self._fix_rotated_subtree_sizes(start_node, SENTINEL);
    }

    /// After a rotation updates the parent chain of a component, this restores
    /// correct subtree sizes along the affected branch until reaching `stop_parent`.
    ///
    /// Arguments:
    /// - `start_node`: the node where the updated branch begins
    /// - `stop_parent`: the parent value at which to stop adjusting
    #[inline(always)]
    fn _fix_rotated_subtree_sizes(&mut self, mut u: usize, stop_parent: usize) {
        let mut p = self.nodes[u].parent;
        while p != stop_parent {
            self.nodes[u].subtree_size -= self.nodes[p].subtree_size;
            self.nodes[p].subtree_size += self.nodes[u].subtree_size;
            u = p;
            p = self.nodes[p].parent;
        }
    }

    fn unlink(&mut self, u: usize, v: usize) -> (usize, i32) {
        let subtree_u_size = self.nodes[u as usize].subtree_size;

        let mut root_v = 0;
        let mut w = v;
        while w != SENTINEL {
            self.nodes[w].subtree_size -= subtree_u_size;
            root_v = w as usize;
            w = self.nodes[w as usize].parent;
        }
        self.nodes[u as usize].parent = SENTINEL;
        (root_v, subtree_u_size)
    }
}

// MARK: DSU specific functions

impl DNDTree {
    fn init_dsu_lists(&mut self) {
        self.l_nodes = (0..self.n).map(|_| Link::new()).collect();
        self.children_head = vec![SENTINEL; self.n];
        self.children_tail = vec![SENTINEL; self.n];
        self.link_parent = vec![SENTINEL; self.n];
        self.roots = (0..self.n).collect();
    }

    #[inline(always)]
    fn unlink_link(&mut self, idx: usize) {
        let parent = self.link_parent[idx];
        if parent == SENTINEL {
            return;
        }

        let prev = self.l_nodes[idx].prev;
        let next = self.l_nodes[idx].next;

        if prev != SENTINEL {
            self.l_nodes[prev].next = next;
        } else {
            self.children_head[parent] = next;
        }

        if next != SENTINEL {
            self.l_nodes[next].prev = prev;
        } else {
            self.children_tail[parent] = prev;
        }

        self.l_nodes[idx].prev = SENTINEL;
        self.l_nodes[idx].next = SENTINEL;
        self.link_parent[idx] = SENTINEL;
    }

    #[inline(always)]
    fn insert_child(&mut self, parent: usize, child: usize) {
        self.unlink_link(child);

        let old_head = self.children_head[parent];
        if old_head == SENTINEL {
            self.children_head[parent] = child;
            self.children_tail[parent] = child;
            self.l_nodes[child].prev = SENTINEL;
            self.l_nodes[child].next = SENTINEL;
        } else {
            self.l_nodes[child].next = old_head;
            self.l_nodes[child].prev = SENTINEL;
            self.l_nodes[old_head].prev = child;
            self.children_head[parent] = child;
        }

        self.link_parent[child] = parent;
    }

    #[inline(always)]
    fn splice_children(&mut self, dst: usize, src: usize) {
        let head = self.children_head[src];
        if head == SENTINEL {
            return;
        }
        let tail = self.children_tail[src];

        let mut cur = head;
        while cur != SENTINEL {
            self.link_parent[cur] = dst;
            cur = self.l_nodes[cur].next;
        }

        let dst_head = self.children_head[dst];
        if dst_head == SENTINEL {
            self.children_head[dst] = head;
            self.children_tail[dst] = tail;
        } else {
            self.l_nodes[tail].next = dst_head;
            self.l_nodes[dst_head].prev = tail;
            self.children_head[dst] = head;
        }

        self.children_head[src] = SENTINEL;
        self.children_tail[src] = SENTINEL;
    }

    #[inline(always)]
    fn get_dsu_root(&mut self, u: usize) -> usize {
        let mut root = u;
        while self.root(root) != root {
            root = self.root(root);
        }

        let mut cur = u;
        while self.root(cur) != root {
            let next = self.root(cur);

            self.unlink_link(cur);
            self.insert_child(root, cur);

            *self.root_mut(cur) = root;
            cur = next;
        }
        root
    }

    /// After a tree edge deletion with no replacement found, this function splits
    /// the DSU structure into two separate components.
    ///
    /// - `small_root`: The root of the detached small subtree (the side that was cut).
    /// - `large_root`: The root of the remaining larger component.
    ///
    /// This function re-parents all nodes in the small subtree to their new roots
    /// and moves the linked lists accordingly.
    //
    //  NOTE: Uses pre-populated self.vec_scratch_nodes from find_replacement.
    //
    #[inline(always)]
    fn remove_subtree_union_find(&mut self, small_root: usize, large_root: usize) {
        let fv = large_root;
        let subtree_nodes = self.vec_scratch_nodes.clone();

        // Detach all small subtree child lists.
        for node in subtree_nodes.iter().copied() {
            let mut cur = self.children_head[node];
            while cur != SENTINEL {
                *self.root_mut(cur) = fv;
                cur = self.l_nodes[cur].next;
            }
            self.splice_children(fv, node);
        }

        // Re-parent small subtree children.
        for node in subtree_nodes {
            *self.root_mut(node) = small_root;
            self.unlink_link(node);
            self.insert_child(small_root, node);
            *self.root_mut(node) = small_root;
        }
    }

    #[inline(always)]
    fn reroot_dsu(&mut self, u: usize, f: usize) {
        *self.root_mut(f) = u;
        self.unlink_link(f);
        self.insert_child(u, f);

        *self.root_mut(u) = u;
        self.unlink_link(u);
        self.insert_child(u, u);
    }

    #[inline(always)]
    fn union_f(&mut self, fu: usize, fv: usize) {
        if fu == fv {
            return;
        }
        *self.root_mut(fu) = fv;
        self.unlink_link(fu);
        self.insert_child(fv, fu);
    }
}

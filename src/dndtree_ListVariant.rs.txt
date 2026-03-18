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
    // for graph
    neighbors: SmallVec<[i32; 4]>,

    // for tree
    pub parent: i32,
    pub subtree_size: i32,

    // for union_find
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
        } else {
            return 1;
        }
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
            return self.get_f(u) == self.get_f(v);
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
            use_union_find: use_union_find,
            compress_links: compress_links,
        }
    }

    fn initialize(&mut self) {
        let n = self.n;
        let use_union_find = self.use_union_find;

        let mut s: Vec<(i32, i32)> = vec![];
        self.used = vec![false; n];
        for i in 0..n {
            let length = self.nodes[i].neighbors.len() as i32;
            s.push((length, -(i as i32)));
        }
        s.sort();

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

        for v in 0..n {
            self.nodes[v].parent = -1;
            self.nodes[v].subtree_size = 1;
        }

        for i in (0..n).rev() {
            let f = (-s[i].1) as usize;
            if self.used[f] {
                continue;
            }
            self.vec_scratch_nodes.clear();
            self.used[f] = true;
            self.vec_scratch_nodes.push(f as i32);

            if use_union_find {
                self.splice_list(f, f);
            }

            let mut s_index = 0;
            while s_index < self.vec_scratch_nodes.len() {
                let p = self.vec_scratch_nodes[s_index];
                for j in 0..self.nodes[p as usize].neighbors.len() {
                    let v = self.nodes[p as usize].neighbors[j] as usize;
                    if !self.used[v] {
                        self.used[v] = true;
                        self.vec_scratch_nodes.push(v as i32);
                        self.nodes[v].parent = p as i32;
                        if use_union_find {
                            self.nodes[v].root = f as i32;
                            self.splice_list(f, v);
                        }
                    }
                }
                s_index += 1;
            }

            let mut i = self.vec_scratch_nodes.len() - 1;
            while i > 0 {
                let q_idx = self.vec_scratch_nodes[i as usize] as usize;
                let p_idx = self.nodes[q_idx].parent as usize;
                self.nodes[p_idx].subtree_size += self.nodes[q_idx].subtree_size;
                i -= 1;
            }

            let mut r: i32 = -1;
            let ss = self.vec_scratch_nodes.len() / 2;
            for i in (0..self.vec_scratch_nodes.len()).rev() {
                if r == -1
                    && self.nodes[self.vec_scratch_nodes[i] as usize].subtree_size as usize > ss
                {
                    r = self.vec_scratch_nodes[i] as i32;
                }
            }
            if r != -1 && r != f as i32 {
                self.reroot(r as usize, f as i32);
            }
        }
        self.used.fill(false);
    }

    fn insert_edge_in_graph(&mut self, u: usize, v: usize) -> bool {
        if u >= self.n || v >= self.n || u == v {
            return false;
        }
        let inserted_u = self.nodes[u].insert_neighbor(v as i32);
        let inserted_v = self.nodes[v].insert_neighbor(u as i32);
        inserted_u == 0 && inserted_v == 0
    }

    fn insert_edge_balanced(&mut self, mut u: usize, mut v: usize) -> i32 {
        let (mut fu, mut fv, mut p, mut pp);
        if !self.use_union_find {
            fu = u;
            while self.nodes[fu].parent != -1 {
                fu = self.nodes[fu].parent as usize;
            }
            fv = v;
            while self.nodes[fv].parent != -1 {
                fv = self.nodes[fv].parent as usize;
            }
        } else {
            fu = self.get_f(u);
            fv = self.get_f(v);
        }

        if fu == fv {
            let mut reshape = false;
            let mut d = 0;
            p = self.nodes[u].parent;
            pp = self.nodes[v].parent;
            while d < MAX_DEPTH {
                if p == -1 {
                    if pp != -1 && self.nodes[pp as usize].parent != -1 {
                        reshape = true;
                        std::mem::swap(&mut u, &mut v);
                        std::mem::swap(&mut p, &mut pp);
                    }
                    break;
                } else if pp == -1 {
                    if p != -1 && self.nodes[p as usize].parent != -1 {
                        reshape = true;
                    }
                    break;
                }
                p = self.nodes[p as usize].parent;
                pp = self.nodes[pp as usize].parent;
                d += 1;
            }

            if reshape {
                let mut dlt = 0;
                while p != -1 {
                    dlt += 1;
                    p = self.nodes[p as usize].parent;
                }

                dlt = dlt / 2 - 1;
                p = u as i32;
                while dlt > 0 {
                    p = self.nodes[p as usize].parent;
                    dlt -= 1;
                }

                pp = self.nodes[p as usize].parent;
                while pp != -1 {
                    self.nodes[pp as usize].subtree_size -= self.nodes[p as usize].subtree_size;
                    pp = self.nodes[pp as usize].parent;
                }

                self.nodes[p as usize].parent = -1;
                self.reroot(u, -1);

                self.nodes[u].parent = v as i32;

                let s = (self.nodes[fu].subtree_size + self.nodes[u].subtree_size) / 2;
                let mut r = -1;
                p = v as i32;
                while p != -1 {
                    self.nodes[p as usize].subtree_size += self.nodes[u].subtree_size;
                    if r == -1 && self.nodes[p as usize].subtree_size > s {
                        r = p;
                    }
                    p = self.nodes[p as usize].parent;
                }
                if r != -1 && r != fu as i32 {
                    self.reroot(r as usize, fu as i32);
                }
            }
            return 0;
        }

        if self.nodes[fu].subtree_size > self.nodes[fv].subtree_size {
            std::mem::swap(&mut u, &mut v);
            std::mem::swap(&mut fu, &mut fv);
        }

        p = self.nodes[u].parent;
        self.nodes[u].parent = v as i32;
        while p != -1 {
            pp = self.nodes[p as usize].parent;
            self.nodes[p as usize].parent = u as i32;
            u = p as usize;
            p = pp;
        }

        let s = (self.nodes[fu].subtree_size + self.nodes[fv].subtree_size) / 2;
        let mut r = -1;
        p = v as i32;
        while p != -1 {
            self.nodes[p as usize].subtree_size += self.nodes[fu].subtree_size;
            if r == -1 && self.nodes[p as usize].subtree_size > s {
                r = p;
            }
            p = self.nodes[p as usize].parent;
        }

        p = self.nodes[u].parent;
        while p != v as i32 {
            self.nodes[u].subtree_size -= self.nodes[p as usize].subtree_size;
            self.nodes[p as usize].subtree_size += self.nodes[u].subtree_size;
            u = p as usize;
            p = self.nodes[u].parent;
        }

        if self.use_union_find {
            self.union_f(fu, fv);
        }

        if r != -1 && r != fv as i32 {
            self.reroot(r as usize, fv as i32);
        }

        1
    }

    fn delete_edge_in_graph(&mut self, u: usize, v: usize) -> bool {
        if u >= self.n || v >= self.n || u == v {
            return false;
        }
        let deleted_u = self.nodes[u].delete_neighbor(v as i32);
        let deleted_v = self.nodes[v].delete_neighbor(u as i32);
        deleted_u == 0 && deleted_v == 0
    }

    fn delete_edge_balanced(&mut self, mut u: usize, mut v: usize) -> i32 {
        if (self.nodes[u].parent != v as i32 && self.nodes[v].parent != u as i32) || u == v {
            return 0;
        }
        if self.nodes[v].parent == u as i32 {
            std::mem::swap(&mut u, &mut v);
        }

        let mut f = v as i32;
        let mut w = v as i32;
        while w != -1 {
            self.nodes[w as usize].subtree_size -= self.nodes[u].subtree_size;
            f = w;
            w = self.nodes[w as usize].parent;
        }

        self.nodes[u].parent = -1;

        let f_usize = f as usize;

        let (ns, nl, need_reroot): (usize, usize, bool) =
            if self.nodes[u].subtree_size > self.nodes[f_usize].subtree_size {
                (f_usize, u, true)
            } else {
                (u, f_usize, false)
            };

        if self.use_union_find && need_reroot {
            self.nodes[f_usize].root = u as i32;
            self.splice_list(f_usize, u);
            self.nodes[u].root = u as i32;
        }

        if self.find_replacement(ns, nl) {
            return 1;
        }

        if self.use_union_find {
            self.remove_subtree_union_find(ns, nl, need_reroot);
        }

        2
    }

    fn find_replacement(&mut self, u: usize, f: usize) -> bool {
        self.vec_scratch_nodes.clear();
        self.vec_scratch_stack.clear();

        self.vec_scratch_nodes.push(u as i32);
        self.vec_scratch_stack.push(u);
        self.used[u] = true;

        let mut i = 0;
        while i < self.vec_scratch_nodes.len() {
            let mut node = self.vec_scratch_nodes[i];
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
                    if !self.used[neighbor as usize] {
                        self.used[neighbor as usize] = true;
                        self.vec_scratch_stack.push(neighbor as usize);
                    }
                    j += 1;
                    continue;
                }

                // Try to build a new path from y upward
                let mut succ = true;
                let mut w = neighbor;
                while w != -1 {
                    if self.used[w as usize] {
                        succ = false;
                        break;
                    }
                    self.used[w as usize] = true;
                    self.vec_scratch_stack.push(w as usize);

                    w = self.nodes[w as usize].parent;
                }
                if !succ {
                    j += 1;
                    continue;
                }

                // Reconnect path from node to neighbor
                let mut p = self.nodes[node as usize].parent;
                self.nodes[node as usize].parent = neighbor as i32;
                while p != -1 {
                    let pp = self.nodes[p as usize].parent;
                    self.nodes[p as usize].parent = node;
                    node = p;
                    p = pp;
                }

                // Compute new root
                let s = (self.nodes[f].subtree_size + self.nodes[u].subtree_size) / 2;
                let mut new_root = None;
                let mut p = neighbor as i32;
                while p != -1 {
                    self.nodes[p as usize].subtree_size += self.nodes[u].subtree_size;
                    if new_root.is_none() && self.nodes[p as usize].subtree_size > s {
                        new_root = Some(p as usize);
                    }
                    p = self.nodes[p as usize].parent;
                }

                // Fix subtree sizes
                let mut p = self.nodes[node as usize].parent;
                while p != neighbor as i32 {
                    self.nodes[node as usize].subtree_size -= self.nodes[p as usize].subtree_size;
                    self.nodes[p as usize].subtree_size += self.nodes[node as usize].subtree_size;
                    node = p;
                    p = self.nodes[p as usize].parent;
                }

                for &k in &self.vec_scratch_stack {
                    self.used[k] = false;
                }

                if new_root.is_some() && new_root != Some(f) {
                    self.reroot(new_root.unwrap(), f as i32);
                }

                return true;
            }
        }

        for &k in &self.vec_scratch_stack {
            self.used[k] = false;
        }

        false
    }

    fn get_f(&mut self, u: usize) -> usize {
        let mut root = u;
        while self.nodes[root].root as usize != root {
            root = self.nodes[root].root as usize;
        }

        if self.compress_links {
            let mut cur = u;
            while self.nodes[cur].root as usize != root {
                let next = self.nodes[cur].root as usize;

                self.isolate_link(cur);
                self.insert_link_to_root(root, cur);

                self.nodes[cur].root = root as i32;
                cur = next;
            }
        } else {
            let mut cur = u;
            while self.nodes[cur].root as usize != root {
                let next = self.nodes[cur].root as usize;
                self.nodes[cur].root = root as i32;
                cur = next;
            }
        }

        root
    }

    fn reroot(&mut self, mut u: usize, f: i32) {
        // Rotate tree: set parents of nodes between u and the old root.
        let mut p = self.nodes[u].parent;
        self.nodes[u].parent = -1;
        while p != -1 {
            let temp = self.nodes[p as usize].parent;
            self.nodes[p as usize].parent = u as i32;
            u = p as usize;
            p = temp;
        }

        // Fix subtree sizes of nodes between u and the old root.
        p = self.nodes[u].parent;
        while p != -1 {
            self.nodes[u].subtree_size -= self.nodes[p as usize].subtree_size;
            self.nodes[p as usize].subtree_size += self.nodes[u].subtree_size;
            u = p as usize;
            p = self.nodes[p as usize].parent;
        }
        if self.use_union_find && f >= 0 {
            self.nodes[f as usize].root = u as i32;
            self.splice_list(f as usize, u);
            self.nodes[u].root = u as i32;
        }
    }
    fn remove_subtree_union_find(&mut self, u: usize, _v: usize, _need_reroot: bool) {
        let detached_root = u;
        self.nodes[u].root = u as i32;

        for &xi in &self.vec_scratch_nodes {
            let x = xi as usize;
            self.nodes[x].root = detached_root as i32;
        }
    }

    fn union_f(&mut self, fu: usize, fv: usize) {
        if fu == fv {
            return;
        }
        self.nodes[fu].root = fv as i32;
        self.splice_list(fu, fv);
    }

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
    fn insert_link_to_root(&mut self, r: usize, child_idx: usize) {
        self.l_nodes[child_idx].prev = None;

        if let Some(old_head) = self.root_head[r] {
            self.l_nodes[child_idx].next = Some(old_head);
            self.l_nodes[old_head].prev = Some(child_idx);
            self.root_head[r] = Some(child_idx);
            // tail unchanged
        } else {
            self.root_head[r] = Some(child_idx);
            self.root_tail[r] = Some(child_idx);
            self.l_nodes[child_idx].next = None;
        }
    }

    // Bulk splice: move entire list from old_root to new_root (append or prepend)
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

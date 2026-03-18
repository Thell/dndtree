use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::vec;

use nohash_hasher::{IntMap, IntSet};
use smallvec::SmallVec;

const MAX_DEPTH: i32 = 32767;

// MARK: LinkNode

#[derive(Clone, Debug)]
struct LinkNode {
    pub v: i32,
    pub prev: Option<Weak<RefCell<LinkNode>>>,
    pub next: Option<Rc<RefCell<LinkNode>>>,
}

impl Drop for LinkNode {
    fn drop(&mut self) {
        // We only care about the 'next' chain.
        // 'prev' is a Weak pointer, so it doesn't trigger recursive drops.
        let mut next = self.next.take();

        while let Some(rc_node) = next {
            // Check if we are the last owner of this node.
            // If strong_count is 1, dropping this Rc will trigger its internal drop.
            // We want to "short-circuit" that by taking its 'next' field here.
            if let Ok(ref_cell) = Rc::try_unwrap(rc_node) {
                // We now own the RefCell. Take its 'next' and continue the loop.
                next = ref_cell.into_inner().next.take();
            } else {
                // Something else still points to this node (e.g. your nodes Vec),
                // so we stop here.
                break;
            }
        }
    }
}

impl LinkNode {
    pub fn new() -> Self {
        LinkNode {
            v: -1,
            prev: None,
            next: None,
        }
    }

    pub fn isolate(&mut self) {
        let tmp_prev = self.prev.take();
        let tmp_next = self.next.take();

        if let Some(ref prev_weak) = tmp_prev {
            if let Some(prev_rc) = prev_weak.upgrade() {
                prev_rc.borrow_mut().next = tmp_next.clone();
            }
        }

        if let Some(ref next_rc) = tmp_next {
            next_rc.borrow_mut().prev = tmp_prev;
        }
    }
}

// MARK: Node

#[derive(Clone, Debug)]
struct Node {
    // for graph
    neighbors: SmallVec<[i32; 4]>,

    // for tree
    parent: i32,
    subtree_size: i32,

    // for union_find
    root: i32,
    children_start: Rc<RefCell<LinkNode>>,
    children_end: Rc<RefCell<LinkNode>>,
}

impl Node {
    fn new() -> Self {
        let children_start = Rc::new(RefCell::new(LinkNode::new()));
        let children_end = Rc::new(RefCell::new(LinkNode::new()));

        children_start.borrow_mut().next = Some(children_end.clone());
        children_end.borrow_mut().prev = Some(Rc::downgrade(&children_start));

        Node {
            neighbors: SmallVec::new(),
            parent: -1,
            subtree_size: 0,
            root: -1,
            children_start,
            children_end,
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

    fn insert_l_node(&self, v: Rc<RefCell<LinkNode>>) {
        let mut v_borrow = v.borrow_mut();
        let start_borrow = self.children_start.borrow();

        v_borrow.next = start_borrow.next.clone();
        v_borrow.prev = Some(Rc::downgrade(&self.children_start));

        if let Some(ref next_node) = start_borrow.next {
            next_node.borrow_mut().prev = Some(Rc::downgrade(&v));
        }

        drop(start_borrow);
        self.children_start.borrow_mut().next = Some(v.clone());
    }

    fn insert_l_nodes(&self, v: &Node) {
        let v_first = v.children_start.borrow().next.clone();
        if let Some(ref first) = v_first {
            if Rc::ptr_eq(first, &v.children_end) || std::ptr::eq(self, v) {
                return;
            }
        }

        let s = v_first.unwrap();
        let t_weak = v.children_end.borrow().prev.clone().unwrap();
        let t = t_weak.upgrade().expect("Reference integrity failure");

        t.borrow_mut().next = self.children_start.borrow().next.clone();
        s.borrow_mut().prev = Some(Rc::downgrade(&self.children_start));

        if let Some(ref next_node) = self.children_start.borrow().next {
            next_node.borrow_mut().prev = Some(Rc::downgrade(&t));
        }

        self.children_start.borrow_mut().next = Some(s);
        v.children_start.borrow_mut().next = Some(v.children_end.clone());

        v.children_end.borrow_mut().prev = Some(Rc::downgrade(&v.children_start));
    }
}

/// DNDTree
#[derive(Clone, Debug)]
pub struct DNDTree {
    n: usize,
    nodes: Vec<Node>,
    l_nodes: Vec<Rc<RefCell<LinkNode>>>,

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
            used: vec![],
            vec_scratch_nodes: vec![],
            vec_scratch_stack: vec![],
            use_union_find,
            compress_links,
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
            self.l_nodes = (0..n)
                .map(|_| Rc::new(RefCell::new(LinkNode::new())))
                .collect();
            for v in 0..n {
                let node_ref_mut = &mut self.nodes[v];

                {
                    let mut l_node = self.l_nodes[v].borrow_mut();
                    l_node.v = v as i32;
                    l_node.prev = None;
                    l_node.next = None;
                }

                node_ref_mut.root = v as i32;

                node_ref_mut.children_start.borrow_mut().next =
                    Some(node_ref_mut.children_end.clone());
                node_ref_mut.children_end.borrow_mut().prev =
                    Some(Rc::downgrade(&node_ref_mut.children_start));

                node_ref_mut.children_start.borrow_mut().prev = None;
                node_ref_mut.children_end.borrow_mut().next = None;
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
                self.nodes[f].insert_l_node(self.l_nodes[f].clone());
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
                            self.nodes[f].insert_l_node(self.l_nodes[v].clone());
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

        let mut f = 0;
        let mut w = v as i32;
        while w != -1 {
            self.nodes[w as usize].subtree_size -= self.nodes[u].subtree_size;
            f = w;
            w = self.nodes[w as usize].parent;
        }

        self.nodes[u].parent = -1;
        let (ns, nl, need_reroot): (usize, usize, bool) =
            if self.nodes[u].subtree_size > self.nodes[f as usize].subtree_size {
                (f as usize, u as usize, true)
            } else {
                (u as usize, f as usize, false)
            };

        if self.use_union_find && need_reroot {
            self.nodes[f as usize].root = u as i32;
            self.l_nodes[f as usize].borrow_mut().isolate();
            self.nodes[u].insert_l_node(self.l_nodes[f as usize].clone());

            self.nodes[u].root = u as i32;
            self.l_nodes[u as usize].borrow_mut().isolate();
            self.nodes[u as usize].insert_l_node(self.l_nodes[u as usize].clone());
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

                self.l_nodes[cur].borrow_mut().isolate();
                self.nodes[root].insert_l_node(self.l_nodes[cur].clone());

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

    // fn get_f(&mut self, u: usize) -> usize {
    //     if self.nodes[u].root != u as i32 {
    //         let f = self.get_f(self.nodes[u].root as usize);
    //         self.nodes[u].root = f as i32;
    //         // self.l_nodes[u].borrow_mut().isolate();
    //         // self.nodes[f].insert_l_node(self.l_nodes[u].clone());
    //     }
    //     self.nodes[u].root as usize
    // }

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
            self.l_nodes[f as usize].borrow_mut().isolate();
            self.nodes[u].insert_l_node(self.l_nodes[f as usize].clone());

            self.nodes[u].root = u as i32;
            self.l_nodes[u as usize].borrow_mut().isolate();
            self.nodes[u].insert_l_node(self.l_nodes[u as usize].clone());
        }
    }

    fn remove_subtree_union_find(&mut self, u: usize, v: usize, _need_reroot: bool) {
        let fv = v;
        for &x in &self.vec_scratch_nodes {
            let mut l_start_next = self.nodes[x as usize].children_start.borrow().next.clone();
            let l_end = self.nodes[x as usize].children_end.clone();

            while let Some(curr) = l_start_next {
                if Rc::ptr_eq(&curr, &l_end) {
                    break;
                }
                let y_v = curr.borrow().v as usize;
                self.nodes[y_v].root = fv as i32;

                l_start_next = curr.borrow().next.clone();
            }

            let (a, b) = if fv < x as usize {
                let (left, right) = self.nodes.split_at_mut(x as usize);
                (&mut left[fv], &right[0])
            } else {
                let (left, right) = self.nodes.split_at_mut(fv);
                (&mut right[0], &left[x as usize])
            };

            a.insert_l_nodes(b);
        }

        for &x in &self.vec_scratch_nodes {
            self.nodes[x as usize].root = u as i32;
            self.l_nodes[x as usize].borrow_mut().isolate();
            self.nodes[u as usize].insert_l_node(self.l_nodes[x as usize].clone());
        }
    }

    fn union_f(&mut self, fu: usize, fv: usize) {
        if fu == fv {
            return;
        }
        self.nodes[fu].root = fv as i32;
        self.l_nodes[fu].borrow_mut().isolate();
        self.nodes[fv].insert_l_node(self.l_nodes[fu].clone());
    }
}

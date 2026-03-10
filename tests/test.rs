use dndtree::DNDTree;
use idtree::IdTree;
use nohash_hasher::{IntMap, IntSet};
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;

fn make_adj_i32(n: usize, edges: &[(usize, usize)]) -> IntMap<i32, IntSet<i32>> {
    let mut adj: IntMap<i32, IntSet<i32>> = IntMap::default();
    for i in 0..n {
        adj.insert(i as i32, IntSet::default());
    }
    for &(u, v) in edges {
        adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
        adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
    }
    adj
}

fn make_adj_usize(n: usize, edges: &[(usize, usize)]) -> IntMap<usize, IntSet<usize>> {
    let mut adj: IntMap<usize, IntSet<usize>> = IntMap::default();
    for i in 0..n {
        adj.insert(i, IntSet::default());
    }
    for &(u, v) in edges {
        adj.get_mut(&u).unwrap().insert(v);
        adj.get_mut(&v).unwrap().insert(u);
    }
    adj
}

fn connected_idtree(tree: &mut IdTree, u: usize, v: usize) -> bool {
    tree.query(u, v)
}

fn connected_dnd(tree: &mut DNDTree, u: usize, v: usize) -> bool {
    tree.query(u, v)
}

#[test]
fn test_basic_insert_delete_query_no_uf() {
    let edges = vec![(0, 1), (1, 2), (2, 3)];
    let adj = make_adj_i32(4, &edges);
    let mut t = DNDTree::new(&adj, false);

    assert!(t.query(0, 3));
    t.delete_edge(1, 2);
    assert!(!t.query(0, 3));
    t.insert_edge(1, 2);
    assert!(t.query(0, 3));
}

#[test]
fn test_basic_insert_delete_query_with_uf() {
    let edges = vec![(0, 1), (1, 2), (2, 3)];
    let adj = make_adj_i32(4, &edges);
    let mut t = DNDTree::new(&adj, true);

    assert!(t.query(0, 3));
    t.delete_edge(1, 2);
    assert!(!t.query(0, 3));
    t.insert_edge(1, 2);
    assert!(t.query(0, 3));
}

#[test]
fn test_unlink_splits_correctly() {
    let edges = vec![(0, 1), (1, 2), (2, 3)];
    let adj = make_adj_i32(4, &edges);
    let mut t = DNDTree::new(&adj, false);

    t.delete_edge(1, 2);
    assert!(t.query(0, 1));
    assert!(!t.query(0, 3));
    assert!(t.query(2, 3));
}

#[test]
fn test_replacement_edge_found() {
    let edges = vec![(0, 1), (1, 2), (2, 3), (0, 3)];
    let adj = make_adj_i32(4, &edges);
    let mut t = DNDTree::new(&adj, false);

    let r = t.delete_edge(1, 2);
    assert_eq!(r, 1);
    assert!(t.query(1, 2));
    assert!(t.query(0, 3));
}

#[test]
fn test_replacement_edge_not_found() {
    let edges = vec![(0, 1), (1, 2), (2, 3)];
    let adj = make_adj_i32(4, &edges);
    let mut t = DNDTree::new(&adj, false);

    let r = t.delete_edge(1, 2);
    assert_eq!(r, 2);
    assert!(!t.query(0, 3));
}

#[test]
fn test_dndtree_matches_idtree_no_uf() {
    let mut rng = StdRng::seed_from_u64(99999);
    let n = 50;
    let mut edges = vec![];

    while edges.len() < 100 {
        let u = rng.random_range(0..n);
        let v = rng.random_range(0..n);
        if u != v {
            edges.push((u, v));
        }
    }

    let adj_dnd = make_adj_i32(n, &edges);
    let adj_id = make_adj_usize(n, &edges);

    for n in adj_dnd.keys() {
        assert_eq!(
            adj_dnd.get(n).unwrap().len(),
            adj_id.get(&(*n as usize)).unwrap().len()
        );
    }

    let mut dnd = DNDTree::new(&adj_dnd, false);
    let mut idt = IdTree::new(&adj_id);

    for _ in 0..200 {
        let u = rng.random_range(0..n);
        let v = rng.random_range(0..n);

        let op = rng.random_range(0..3);
        match op {
            0 => {
                dnd.insert_edge(u, v);
                idt.insert_edge(u, v);
            }
            1 => {
                dnd.delete_edge(u, v);
                idt.delete_edge(u, v);
            }
            _ => {}
        }

        for _ in 0..20 {
            let a = rng.random_range(0..n);
            let b = rng.random_range(0..n);
            assert_eq!(
                connected_dnd(&mut dnd, a, b),
                connected_idtree(&mut idt, a, b)
            );
        }
    }
}

#[test]
fn test_dndtree_matches_idtree_with_uf() {
    let mut rng = StdRng::seed_from_u64(99999);
    let n = 50;
    let mut edges = vec![];

    for _ in 0..100 {
        let u = rng.random_range(0..n);
        let v = rng.random_range(0..n);
        if u != v {
            edges.push((u, v));
        }
    }

    let adj_dnd = make_adj_i32(n, &edges);
    let adj_id = make_adj_usize(n, &edges);

    let mut dnd = DNDTree::new(&adj_dnd, true);
    let mut idt = IdTree::new(&adj_id);

    for _ in 0..200 {
        let u = rng.random_range(0..n);
        let v = rng.random_range(0..n);

        let op = rng.random_range(0..3);
        match op {
            0 => {
                dnd.insert_edge(u, v);
                idt.insert_edge(u, v);
            }
            1 => {
                dnd.delete_edge(u, v);
                idt.delete_edge(u, v);
            }
            _ => {}
        }

        for _ in 0..20 {
            let a = rng.random_range(0..n);
            let b = rng.random_range(0..n);
            assert_eq!(
                connected_dnd(&mut dnd, a, b),
                connected_idtree(&mut idt, a, b)
            );
        }
    }
}

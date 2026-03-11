use divan::Bencher;
use dndtree::DNDTree;
use nohash_hasher::{IntMap, IntSet};

const ARGS: &[usize] = &[1_000, 10_000, 100_000, 200_000];

fn make_adj(n: usize) -> IntMap<i32, IntSet<i32>> {
    let mut adj: IntMap<i32, IntSet<i32>> = IntMap::default();
    for i in 0..n {
        adj.insert(i as i32, IntSet::default());
    }
    adj
}

fn make_edges(n: usize) -> Vec<(usize, usize)> {
    let mut edges = Vec::with_capacity(n);
    for i in 0..n {
        let u = i % n;
        let v = (i * 7 + 13) % n;
        if u != v {
            edges.push((u, v));
        }
    }
    edges
}

mod with_union_find {
    use super::*;

    #[divan::bench(args = ARGS)]
    fn bench_build_from_adj(bencher: Bencher, n: usize) {
        let mut adj = make_adj(n);
        let edges = make_edges(n);

        // populate adjacency list once
        for &(u, v) in &edges {
            adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
            adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
        }

        bencher.with_inputs(|| adj.clone()).bench_refs(|adj| {
            let _ = DNDTree::new(adj, true);
        });
    }

    #[divan::bench(args = ARGS)]
    fn bench_insert(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let edges = make_edges(n);
        let tree = DNDTree::new(&adj, true);

        bencher
            .with_inputs(|| (edges.clone(), tree.clone()))
            .bench_local_refs(|(edges, tree)| {
                for (u, v) in edges {
                    tree.insert_edge(*u, *v);
                }
            });
    }

    #[divan::bench(args = [1_000, 10_000, 100_000])]
    fn bench_query(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let mut tree = DNDTree::new(&adj, true);
        let edges = make_edges(n);

        // populate once
        for &(u, v) in &edges {
            tree.insert_edge(u, v);
        }

        bencher.bench_local(move || {
            for &(u, v) in &edges {
                let _ = tree.query(u, v);
            }
        });
    }

    #[divan::bench(args = ARGS)]
    fn bench_delete(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let mut tree = DNDTree::new(&adj, true);
        let edges = make_edges(n);

        // populate once
        for &(u, v) in &edges {
            tree.insert_edge(u, v);
        }

        bencher
            .with_inputs(|| (edges.clone(), tree.clone()))
            .bench_local_refs(|(edges, tree)| {
                for (u, v) in edges {
                    tree.delete_edge(*u, *v);
                }
            });
    }
}

mod without_union_find {
    use super::*;

    #[divan::bench(args = ARGS)]
    fn bench_build_from_adj(bencher: Bencher, n: usize) {
        let mut adj = make_adj(n);
        let edges = make_edges(n);

        // populate adjacency list once
        for &(u, v) in &edges {
            adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
            adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
        }

        bencher.with_inputs(|| adj.clone()).bench_refs(|adj| {
            let _ = DNDTree::new(adj, false);
        });
    }

    #[divan::bench(args = ARGS)]
    fn bench_insert(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let edges = make_edges(n);
        let tree = DNDTree::new(&adj, false);

        bencher
            .with_inputs(|| (edges.clone(), tree.clone()))
            .bench_local_refs(|(edges, tree)| {
                for (u, v) in edges {
                    tree.insert_edge(*u, *v);
                }
            });
    }

    #[divan::bench(args = [1_000, 10_000, 100_000])]
    fn bench_query(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let mut tree = DNDTree::new(&adj, false);
        let edges = make_edges(n);

        // populate once
        for &(u, v) in &edges {
            tree.insert_edge(u, v);
        }

        bencher.bench_local(move || {
            for &(u, v) in &edges {
                let _ = tree.query(u, v);
            }
        });
    }

    #[divan::bench(args = ARGS)]
    fn bench_delete(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let mut tree = DNDTree::new(&adj, false);
        let edges = make_edges(n);

        // populate once
        for &(u, v) in &edges {
            tree.insert_edge(u, v);
        }

        bencher
            .with_inputs(|| (edges.clone(), tree.clone()))
            .bench_local_refs(|(edges, tree)| {
                for (u, v) in edges {
                    tree.delete_edge(*u, *v);
                }
            });
    }
}

fn main() {
    divan::main();
}

use divan::Bencher;
use dndtree::DNDTree;
use nohash_hasher::{IntMap, IntSet};
use rand::prelude::SliceRandom;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

const ARGS: &[usize] = &[1_000, 2_000, 10_000, 20_000, 100_000, 200_000];
const SAMPLE_COUNT: u32 = 10;
const QUERY_FACTOR: f64 = 0.05;

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

fn make_edges_for_nodes(node_count: usize, edge_count: usize) -> Vec<(usize, usize)> {
    let mut edges = Vec::with_capacity(edge_count);
    for i in 0..edge_count {
        let u = i % node_count;
        let v = (i * 7 + 13) % node_count;
        if u != v {
            edges.push((u, v));
        }
    }
    edges
}

fn make_caterpillar_graph(
    n: usize,
    spine_length_ratio: f64, // 0.1 = short spine, 0.5 = half nodes on spine
    extra_edges_ratio: f64,  // how many additional random chords
) -> IntMap<i32, IntSet<i32>> {
    let mut adj: IntMap<i32, IntSet<i32>> = IntMap::default();
    for i in 0..n as i32 {
        adj.insert(i, IntSet::default());
    }

    let mut rng = StdRng::seed_from_u64(42);

    // 1. Create the long spine (backbone path)
    let spine_len = (n as f64 * spine_length_ratio).max(10.0).min(n as f64) as usize;
    let mut spine = Vec::with_capacity(spine_len);
    for i in 0..spine_len {
        spine.push(i as i32);
        if i > 0 {
            let prev = spine[i - 1];
            adj.get_mut(&prev).unwrap().insert(i as i32);
            adj.get_mut(&(i as i32)).unwrap().insert(prev);
        }
    }

    // 2. Attach remaining nodes as leaves or small trees to the spine
    let mut next_node = spine_len as i32;
    while next_node < n as i32 {
        // Pick random spine node to attach to
        let attach_to = spine[rng.random_range(0..spine.len())];

        // Attach a small chain (1–4 nodes) to make subtrees deeper
        let chain_len = rng.random_range(1..=4);
        let mut prev = attach_to;
        for _ in 0..chain_len {
            if next_node >= n as i32 {
                break;
            }
            adj.get_mut(&prev).unwrap().insert(next_node);
            adj.get_mut(&next_node).unwrap().insert(prev);
            prev = next_node;
            next_node += 1;
        }
    }

    // 3. Add a few random chords (keep connectivity high but allow splits)
    let extra_count = (n as f64 * extra_edges_ratio) as usize;
    for _ in 0..extra_count {
        let u = rng.random_range(0..n as i32);
        let v = rng.random_range(0..n as i32);
        if u != v && !adj.get(&u).unwrap().contains(&v) {
            adj.get_mut(&u).unwrap().insert(v);
            adj.get_mut(&v).unwrap().insert(u);
        }
    }

    adj
}

fn make_random_recursive_tree(n: usize, extra_chords_ratio: f64) -> IntMap<i32, IntSet<i32>> {
    let mut adj: IntMap<i32, IntSet<i32>> = IntMap::default();
    for i in 0..n as i32 {
        adj.insert(i, IntSet::default());
    }

    let mut rng = StdRng::seed_from_u64(42);

    // Root is 0
    let mut parents = vec![-1i32; n];
    parents[0] = 0; // self-root

    // Build recursive tree: each new node attaches to a random existing node
    for i in 1..n as i32 {
        let parent_idx = rng.random_range(0..i as usize);
        let parent = parent_idx as i32;

        adj.get_mut(&parent).unwrap().insert(i);
        adj.get_mut(&i).unwrap().insert(parent);

        parents[i as usize] = parent;
    }

    // Add extra random chords (keep graph connected but allow splits)
    let extra_count = (n as f64 * extra_chords_ratio) as usize;
    for _ in 0..extra_count {
        let u = rng.random_range(0..n as i32);
        let v = rng.random_range(0..n as i32);
        if u != v && !adj.get(&u).unwrap().contains(&v) {
            adj.get_mut(&u).unwrap().insert(v);
            adj.get_mut(&v).unwrap().insert(u);
        }
    }

    adj
}

mod with_union_find_and_compression {
    use rand::RngExt;

    use super::*;

    const USE_DSU: bool = true;
    const COMPRESS_LINKS: bool = true;

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn bench_build_from_adj(bencher: Bencher, n: usize) {
        let mut adj = make_adj(n);
        let edges = make_edges(n);

        // populate adjacency list once
        for &(u, v) in &edges {
            adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
            adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
        }

        bencher.with_inputs(|| adj.clone()).bench_refs(|adj| {
            let _ = DNDTree::new(adj, USE_DSU, COMPRESS_LINKS);
        });
    }

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn bench_insert(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let edges = make_edges(n);
        let tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

        bencher
            .with_inputs(|| (edges.clone(), tree.clone()))
            .bench_local_refs(|(edges, tree)| {
                for (u, v) in edges {
                    tree.insert_edge(*u, *v);
                }
            });
    }

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn bench_query(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);
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

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn bench_delete(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);
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

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn mixed_ops(bencher: Bencher, n: usize) {
        let mut edges = make_edges(n * 2);
        let mut rng = StdRng::seed_from_u64(12345);
        edges.shuffle(&mut rng);

        let (present, absent) = edges.split_at_mut(n);

        let mut adj = make_adj(n * 2);
        for &(u, v) in present.iter() {
            adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
            adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
        }

        bencher.bench_local(move || {
            let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

            for i in 0..n {
                let (du, dv) = present[i];
                tree.delete_edge(du, dv);

                let (qu, qv) = present[i % present.len()];
                let _ = tree.query(qu, qv);

                let (iu, iv) = absent[i];
                tree.insert_edge(iu, iv);
            }
        });
    }

    #[divan::bench(args = ARGS, sample_count = 1)]
    fn mixed_ops_query_heavy(bencher: Bencher, n: usize) {
        let mut edges = make_edges_for_nodes(n, n * 2);
        let mut rng = StdRng::seed_from_u64(12345);
        edges.shuffle(&mut rng);

        let (present_edges, absent_edges) = edges.split_at(n);

        let mut adj = make_adj(n);
        for &(u, v) in present_edges.iter() {
            adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
            adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
        }

        // Pre-select random endpoint pairs (not edges)
        let num_query_pairs = (QUERY_FACTOR * n as f64) as usize;
        let mut query_pairs = Vec::with_capacity(num_query_pairs);
        for _ in 0..num_query_pairs {
            let qu = rng.random_range(0..n);
            let qv = rng.random_range(0..n);
            query_pairs.push((qu, qv));
        }

        bencher.bench_local(move || {
            let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

            let mut present: Vec<usize> = (0..n).collect();
            let mut absent: Vec<usize> = (0..n).collect();

            for i in 0..n {
                let pi = present[i];
                let (du, dv) = present_edges[pi];
                tree.delete_edge(du, dv);

                for &(qu, qv) in &query_pairs {
                    let _ = tree.query(qu, qv);
                }

                let ai = absent[i];
                let (iu, iv) = absent_edges[ai];
                tree.insert_edge(iu, iv);

                present[i] = ai;
                absent[i] = pi;
            }
        });
    }

    #[divan::bench(args = ARGS, sample_count = 1)]
    fn mixed_ops_query_heavy_catgraph(bencher: Bencher, n: usize) {
        let mut edges = make_edges_for_nodes(n, n * 2);
        let mut rng = StdRng::seed_from_u64(12345);
        edges.shuffle(&mut rng);

        let (present_edges, absent_edges) = edges.split_at(n);

        // Replace the adj creation line in mixed_ops_query_heavy
        let adj = make_caterpillar_graph(n, 0.3, 0.05); // spine ~30% of nodes, 5% extra chords

        // Pre-select random endpoint pairs (not edges)
        let num_query_pairs = (QUERY_FACTOR * n as f64) as usize;
        let mut query_pairs = Vec::with_capacity(num_query_pairs);
        for _ in 0..num_query_pairs {
            let qu = rng.random_range(0..n);
            let qv = rng.random_range(0..n);
            query_pairs.push((qu, qv));
        }

        bencher.bench_local(move || {
            let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

            let mut present: Vec<usize> = (0..n).collect();
            let mut absent: Vec<usize> = (0..n).collect();

            for i in 0..n {
                let pi = present[i];
                let (du, dv) = present_edges[pi];
                tree.delete_edge(du, dv);

                for &(qu, qv) in &query_pairs {
                    let _ = tree.query(qu, qv);
                }

                let ai = absent[i];
                let (iu, iv) = absent_edges[ai];
                tree.insert_edge(iu, iv);

                present[i] = ai;
                absent[i] = pi;
            }
        });
    }

    #[divan::bench(args = ARGS, sample_count = 1)]
    fn mixed_ops_query_heavy_tree(bencher: Bencher, n: usize) {
        let mut edges = make_edges_for_nodes(n, n * 2);
        let mut rng = StdRng::seed_from_u64(12345);
        edges.shuffle(&mut rng);

        let (present_edges, absent_edges) = edges.split_at(n);

        let adj = make_random_recursive_tree(n, 0.05); // or 0.1 for more chords

        // Pre-select random endpoint pairs (not edges)
        let num_query_pairs = (QUERY_FACTOR * n as f64) as usize;
        let mut query_pairs = Vec::with_capacity(num_query_pairs);
        for _ in 0..num_query_pairs {
            let qu = rng.random_range(0..n);
            let qv = rng.random_range(0..n);
            query_pairs.push((qu, qv));
        }

        bencher.bench_local(move || {
            let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

            let mut present: Vec<usize> = (0..n).collect();
            let mut absent: Vec<usize> = (0..n).collect();

            for i in 0..n {
                let pi = present[i];
                let (du, dv) = present_edges[pi];
                tree.delete_edge(du, dv);

                for &(qu, qv) in &query_pairs {
                    let _ = tree.query(qu, qv);
                }

                let ai = absent[i];
                let (iu, iv) = absent_edges[ai];
                tree.insert_edge(iu, iv);

                present[i] = ai;
                absent[i] = pi;
            }
        });
    }
}

mod with_union_find_no_compression {
    use rand::RngExt;

    use super::*;

    const USE_DSU: bool = true;
    const COMPRESS_LINKS: bool = false;

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn bench_build_from_adj(bencher: Bencher, n: usize) {
        let mut adj = make_adj(n);
        let edges = make_edges(n);

        // populate adjacency list once
        for &(u, v) in &edges {
            adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
            adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
        }

        bencher.with_inputs(|| adj.clone()).bench_refs(|adj| {
            let _ = DNDTree::new(adj, USE_DSU, COMPRESS_LINKS);
        });
    }

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn bench_insert(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let edges = make_edges(n);
        let tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

        bencher
            .with_inputs(|| (edges.clone(), tree.clone()))
            .bench_local_refs(|(edges, tree)| {
                for (u, v) in edges {
                    tree.insert_edge(*u, *v);
                }
            });
    }

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn bench_query(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);
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

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn bench_delete(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);
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

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn mixed_ops(bencher: Bencher, n: usize) {
        let mut edges = make_edges(n * 2);
        let mut rng = StdRng::seed_from_u64(12345);
        edges.shuffle(&mut rng);

        let (present, absent) = edges.split_at_mut(n);

        let mut adj = make_adj(n * 2);
        for &(u, v) in present.iter() {
            adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
            adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
        }

        bencher.bench_local(move || {
            let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

            for i in 0..n {
                let (du, dv) = present[i];
                tree.delete_edge(du, dv);

                let (qu, qv) = present[i % present.len()];
                let _ = tree.query(qu, qv);

                let (iu, iv) = absent[i];
                tree.insert_edge(iu, iv);
            }
        });
    }

    #[divan::bench(args = ARGS, sample_count = 1)]
    fn mixed_ops_query_heavy(bencher: Bencher, n: usize) {
        let mut edges = make_edges_for_nodes(n, n * 2);
        let mut rng = StdRng::seed_from_u64(12345);
        edges.shuffle(&mut rng);

        let (present_edges, absent_edges) = edges.split_at(n);

        let mut adj = make_adj(n);
        for &(u, v) in present_edges.iter() {
            adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
            adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
        }

        // Pre-select random endpoint pairs (not edges)
        let num_query_pairs = (QUERY_FACTOR * n as f64) as usize;
        let mut query_pairs = Vec::with_capacity(num_query_pairs);
        for _ in 0..num_query_pairs {
            let qu = rng.random_range(0..n);
            let qv = rng.random_range(0..n);
            query_pairs.push((qu, qv));
        }

        bencher.bench_local(move || {
            let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

            let mut present: Vec<usize> = (0..n).collect();
            let mut absent: Vec<usize> = (0..n).collect();

            for i in 0..n {
                let pi = present[i];
                let (du, dv) = present_edges[pi];
                tree.delete_edge(du, dv);

                for &(qu, qv) in &query_pairs {
                    let _ = tree.query(qu, qv);
                }

                let ai = absent[i];
                let (iu, iv) = absent_edges[ai];
                tree.insert_edge(iu, iv);

                present[i] = ai;
                absent[i] = pi;
            }
        });
    }

    #[divan::bench(args = ARGS, sample_count = 1)]
    fn mixed_ops_query_heavy_catgraph(bencher: Bencher, n: usize) {
        let mut edges = make_edges_for_nodes(n, n * 2);
        let mut rng = StdRng::seed_from_u64(12345);
        edges.shuffle(&mut rng);

        let (present_edges, absent_edges) = edges.split_at(n);

        // Replace the adj creation line in mixed_ops_query_heavy
        let adj = make_caterpillar_graph(n, 0.3, 0.05); // spine ~30% of nodes, 5% extra chords

        // Pre-select random endpoint pairs (not edges)
        let num_query_pairs = (QUERY_FACTOR * n as f64) as usize;
        let mut query_pairs = Vec::with_capacity(num_query_pairs);
        for _ in 0..num_query_pairs {
            let qu = rng.random_range(0..n);
            let qv = rng.random_range(0..n);
            query_pairs.push((qu, qv));
        }

        bencher.bench_local(move || {
            let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

            let mut present: Vec<usize> = (0..n).collect();
            let mut absent: Vec<usize> = (0..n).collect();

            for i in 0..n {
                let pi = present[i];
                let (du, dv) = present_edges[pi];
                tree.delete_edge(du, dv);

                for &(qu, qv) in &query_pairs {
                    let _ = tree.query(qu, qv);
                }

                let ai = absent[i];
                let (iu, iv) = absent_edges[ai];
                tree.insert_edge(iu, iv);

                present[i] = ai;
                absent[i] = pi;
            }
        });
    }

    #[divan::bench(args = ARGS, sample_count = 1)]
    fn mixed_ops_query_heavy_tree(bencher: Bencher, n: usize) {
        let mut edges = make_edges_for_nodes(n, n * 2);
        let mut rng = StdRng::seed_from_u64(12345);
        edges.shuffle(&mut rng);

        let (present_edges, absent_edges) = edges.split_at(n);

        let adj = make_random_recursive_tree(n, 0.05); // or 0.1 for more chords

        // Pre-select random endpoint pairs (not edges)
        let num_query_pairs = (QUERY_FACTOR * n as f64) as usize;
        let mut query_pairs = Vec::with_capacity(num_query_pairs);
        for _ in 0..num_query_pairs {
            let qu = rng.random_range(0..n);
            let qv = rng.random_range(0..n);
            query_pairs.push((qu, qv));
        }

        bencher.bench_local(move || {
            let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

            let mut present: Vec<usize> = (0..n).collect();
            let mut absent: Vec<usize> = (0..n).collect();

            for i in 0..n {
                let pi = present[i];
                let (du, dv) = present_edges[pi];
                tree.delete_edge(du, dv);

                for &(qu, qv) in &query_pairs {
                    let _ = tree.query(qu, qv);
                }

                let ai = absent[i];
                let (iu, iv) = absent_edges[ai];
                tree.insert_edge(iu, iv);

                present[i] = ai;
                absent[i] = pi;
            }
        });
    }
}

mod without_union_find {
    use rand::RngExt;

    use super::*;

    const USE_DSU: bool = false;
    const COMPRESS_LINKS: bool = false;

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn bench_build_from_adj(bencher: Bencher, n: usize) {
        let mut adj = make_adj(n);
        let edges = make_edges(n);

        // populate adjacency list once
        for &(u, v) in &edges {
            adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
            adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
        }

        bencher.with_inputs(|| adj.clone()).bench_refs(|adj| {
            let _ = DNDTree::new(adj, USE_DSU, COMPRESS_LINKS);
        });
    }

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn bench_insert(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let edges = make_edges(n);
        let tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

        bencher
            .with_inputs(|| (edges.clone(), tree.clone()))
            .bench_local_refs(|(edges, tree)| {
                for (u, v) in edges {
                    tree.insert_edge(*u, *v);
                }
            });
    }

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn bench_query(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);
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

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn bench_delete(bencher: Bencher, n: usize) {
        let adj = make_adj(n);
        let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);
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

    #[divan::bench(args = ARGS, sample_count = SAMPLE_COUNT)]
    fn mixed_ops(bencher: Bencher, n: usize) {
        let mut edges = make_edges(n * 2);
        let mut rng = StdRng::seed_from_u64(12345);
        edges.shuffle(&mut rng);

        let (present, absent) = edges.split_at_mut(n);

        let mut adj = make_adj(n * 2);
        for &(u, v) in present.iter() {
            adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
            adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
        }

        bencher.bench_local(move || {
            let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

            for i in 0..n {
                let (du, dv) = present[i];
                tree.delete_edge(du, dv);

                let (qu, qv) = present[i % present.len()];
                let _ = tree.query(qu, qv);

                let (iu, iv) = absent[i];
                tree.insert_edge(iu, iv);
            }
        });
    }

    #[divan::bench(args = ARGS, sample_count = 1)]
    fn mixed_ops_query_heavy(bencher: Bencher, n: usize) {
        let mut edges = make_edges_for_nodes(n, n * 2);
        let mut rng = StdRng::seed_from_u64(12345);
        edges.shuffle(&mut rng);

        let (present_edges, absent_edges) = edges.split_at(n);

        let mut adj = make_adj(n);
        for &(u, v) in present_edges.iter() {
            adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
            adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
        }

        // Pre-select random endpoint pairs (not edges)
        let num_query_pairs = (QUERY_FACTOR * n as f64) as usize;
        let mut query_pairs = Vec::with_capacity(num_query_pairs);
        for _ in 0..num_query_pairs {
            let qu = rng.random_range(0..n);
            let qv = rng.random_range(0..n);
            query_pairs.push((qu, qv));
        }

        bencher.bench_local(move || {
            let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

            let mut present: Vec<usize> = (0..n).collect();
            let mut absent: Vec<usize> = (0..n).collect();

            for i in 0..n {
                let pi = present[i];
                let (du, dv) = present_edges[pi];
                tree.delete_edge(du, dv);

                for &(qu, qv) in &query_pairs {
                    let _ = tree.query(qu, qv);
                }

                let ai = absent[i];
                let (iu, iv) = absent_edges[ai];
                tree.insert_edge(iu, iv);

                present[i] = ai;
                absent[i] = pi;
            }
        });
    }

    #[divan::bench(args = ARGS, sample_count = 1)]
    fn mixed_ops_query_heavy_catgraph(bencher: Bencher, n: usize) {
        let mut edges = make_edges_for_nodes(n, n * 2);
        let mut rng = StdRng::seed_from_u64(12345);
        edges.shuffle(&mut rng);

        let (present_edges, absent_edges) = edges.split_at(n);

        // Replace the adj creation line in mixed_ops_query_heavy
        let adj = make_caterpillar_graph(n, 0.3, 0.05); // spine ~30% of nodes, 5% extra chords

        // Pre-select random endpoint pairs (not edges)
        let num_query_pairs = (QUERY_FACTOR * n as f64) as usize;
        let mut query_pairs = Vec::with_capacity(num_query_pairs);
        for _ in 0..num_query_pairs {
            let qu = rng.random_range(0..n);
            let qv = rng.random_range(0..n);
            query_pairs.push((qu, qv));
        }

        bencher.bench_local(move || {
            let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

            let mut present: Vec<usize> = (0..n).collect();
            let mut absent: Vec<usize> = (0..n).collect();

            for i in 0..n {
                let pi = present[i];
                let (du, dv) = present_edges[pi];
                tree.delete_edge(du, dv);

                for &(qu, qv) in &query_pairs {
                    let _ = tree.query(qu, qv);
                }

                let ai = absent[i];
                let (iu, iv) = absent_edges[ai];
                tree.insert_edge(iu, iv);

                present[i] = ai;
                absent[i] = pi;
            }
        });
    }

    #[divan::bench(args = ARGS, sample_count = 1)]
    fn mixed_ops_query_heavy_tree(bencher: Bencher, n: usize) {
        let mut edges = make_edges_for_nodes(n, n * 2);
        let mut rng = StdRng::seed_from_u64(12345);
        edges.shuffle(&mut rng);

        let (present_edges, absent_edges) = edges.split_at(n);

        // Replace the adj creation line in mixed_ops_query_heavy
        let adj = make_random_recursive_tree(n, 0.05); // or 0.1 for more chords

        // Pre-select random endpoint pairs (not edges)
        let num_query_pairs = (QUERY_FACTOR * n as f64) as usize;
        let mut query_pairs = Vec::with_capacity(num_query_pairs);
        for _ in 0..num_query_pairs {
            let qu = rng.random_range(0..n);
            let qv = rng.random_range(0..n);
            query_pairs.push((qu, qv));
        }

        bencher.bench_local(move || {
            let mut tree = DNDTree::new(&adj, USE_DSU, COMPRESS_LINKS);

            let mut present: Vec<usize> = (0..n).collect();
            let mut absent: Vec<usize> = (0..n).collect();

            for i in 0..n {
                let pi = present[i];
                let (du, dv) = present_edges[pi];
                tree.delete_edge(du, dv);

                for &(qu, qv) in &query_pairs {
                    let _ = tree.query(qu, qv);
                }

                let ai = absent[i];
                let (iu, iv) = absent_edges[ai];
                tree.insert_edge(iu, iv);

                present[i] = ai;
                absent[i] = pi;
            }
        });
    }
}

fn main() {
    divan::main();
}

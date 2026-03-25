#![allow(dead_code)]
use dndtree::DNDTree;
use nohash_hasher::{IntMap, IntSet};
use rand::SeedableRng;
use rand::prelude::SliceRandom;
use rand::rngs::StdRng;

const QUERY_FACTOR: usize = 10;

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

#[inline(never)]
fn mixed_ops(n: usize, use_uf: bool) {
    let mut edges = make_edges(n * 2);
    let mut rng = StdRng::seed_from_u64(12345);
    edges.shuffle(&mut rng);

    let (present, absent) = edges.split_at_mut(n);

    let mut adj = make_adj(n * 2);
    for &(u, v) in present.iter() {
        adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
        adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
    }

    let mut tree = DNDTree::new(&adj, use_uf);
    for i in 0..n {
        let (du, dv) = present[i];
        tree.delete_edge(du, dv);

        let (qu, qv) = present[i % present.len()];
        let _ = tree.query(qu, qv);

        let (iu, iv) = absent[i];
        tree.insert_edge(iu, iv);
    }
}

fn mixed_ops_query_heavy(n: usize, use_uf: bool) {
    let mut edges = make_edges_for_nodes(n, n * 2);
    let mut rng = StdRng::seed_from_u64(12345);
    edges.shuffle(&mut rng);

    let (present_edges, absent_edges) = edges.split_at(n);

    let mut adj = make_adj(n);
    for &(u, v) in present_edges.iter() {
        adj.get_mut(&(u as i32)).unwrap().insert(v as i32);
        adj.get_mut(&(v as i32)).unwrap().insert(u as i32);
    }

    let mut tree = DNDTree::new(&adj, use_uf);

    let mut present: Vec<usize> = (0..n).collect();
    let mut absent: Vec<usize> = (0..n).collect();

    for i in 0..n {
        let pi = present[i];
        let (du, dv) = present_edges[pi];
        tree.delete_edge(du, dv);

        for q in 0..n / QUERY_FACTOR {
            let qi = present[(i + q) % n];
            let (qu, qv) = present_edges[qi];
            let _ = tree.query(qu, qv);
        }

        let ai = absent[i];
        let (iu, iv) = absent_edges[ai];
        tree.insert_edge(iu, iv);

        present[i] = ai;
        absent[i] = pi;
    }
}

// Take argv arguments for n and for use_uf
fn main() {
    use std::env;
    let args: Vec<String> = env::args().collect();

    let n: usize = args[1].parse().unwrap();
    let use_uf: bool = args[2].parse().unwrap();

    let start_time = std::time::Instant::now();
    mixed_ops(n, use_uf);
    // mixed_ops_query_heavy(n, use_uf);
    let elapsed = start_time.elapsed();
    println!("{} took {} microseconds", n, elapsed.as_micros());
    // Wait for user input
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
}

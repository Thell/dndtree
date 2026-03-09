# DND-Tree

A Rust implementation of the **DND‑Tree** dynamic connectivity data structure from:

**_Constant-time Connectivity Querying in Dynamic Graphs_**,  
Proceedings of the ACM on Management of Data, Volume 2, Issue 6  
Article No.: 230, Pages 1 - 23  
<https://dl.acm.org/doi/abs/10.1145/3698805>

The Improved D-Tree (ID-Tree) is an improvement on the D-Tree data structure from:

**_Dynamic Spanning Trees for Connectivity Queries on Fully-dynamic Undirected Graphs._**,
Proc. VLDB Endow. 15, 11 (2022), 3263–3276
<https://www.vldb.org/pvldb/vol15/p3263-chen.pdf>

The implementation is fully safe Rust.

## Algorithmic Complexity

| Operation          | DND‑Tree    | D‑Tree                                |
|--------------------|-------------|---------------------------------------|
| Query processing   | $O(\alpha)$ | $O(h)$                                |
| Edge insertion     | $O(h)$      | $O(h \cdot \text{nbr}_\text{update})$ |
| Edge deletion      | $O(h)$      | $O(h^2 \cdot \text{nbr}_\text{scan})$ |

Where:

- $\alpha$ is the inverse Ackermann function, a small constant ($\alpha$ < 5)
- $h$ is the average vertex depth in the spanning tree.
- $\text{nbr}_\text{update}$ is the time to insert a vertex into neighbors of a vertex or to
 delete a vertex from neighbors of a vertex.
- $\text{nbr}_\text{scan}$ is the time to scan all neighbors of a vertex.

## Performance Characteristics

```
bench                       fastest       │ slowest       │ median        │ mean          │ samples │ iters
├─ with_union_find                        │               │               │               │         │
│  ├─ bench_build_from_adj¹               │               │               │               │         │
│  │  ├─ 10000              2.744 ms      │ 6.425 ms      │ 2.979 ms      │ 3.329 ms      │ 100     │ 100
│  │  ├─ 100000             43.65 ms      │ 69.96 ms      │ 45.4 ms       │ 45.88 ms      │ 100     │ 100
│  │  ╰─ 500000             246.5 ms      │ 276.5 ms      │ 252.5 ms      │ 253.3 ms      │ 100     │ 100
│  ├─ bench_delete                        │               │               │               │         │
│  │  ├─ 10000              3.498 ms      │ 9.662 ms      │ 3.687 ms      │ 4.012 ms      │ 100     │ 100
│  │  ├─ 100000             133.5 ms      │ 159 ms        │ 139.1 ms      │ 139.7 ms      │ 100     │ 100
│  │  ╰─ 500000             1.238 s       │ 1.274 s       │ 1.253 s       │ 1.254 s       │ 100     │ 100
│  ├─ bench_insert                        │               │               │               │         │
│  │  ├─ 10000              1.404 ms      │ 3.957 ms      │ 1.487 ms      │ 1.688 ms      │ 100     │ 100
│  │  ├─ 100000             47.62 ms      │ 91.72 ms      │ 51.07 ms      │ 51.58 ms      │ 100     │ 100
│  │  ╰─ 500000             477.2 ms      │ 524.1 ms      │ 488.9 ms      │ 489.6 ms      │ 100     │ 100
│  ╰─ bench_query                         │               │               │               │         │
│     ├─ 10000              62.59 µs      │ 147.2 µs      │ 63.19 µs      │ 64.32 µs      │ 100     │ 100
│     ├─ 100000             617.1 µs      │ 3.168 ms      │ 702 µs        │ 770.1 µs      │ 100     │ 100
│     ╰─ 500000             5.563 ms      │ 25.66 ms      │ 11.92 ms      │ 12.03 ms      │ 100     │ 100
╰─ without_union_find                     │               │               │               │         │
   ├─ bench_build_from_adj¹               │               │               │               │         │
   │  ├─ 10000              2.026 ms      │ 5.445 ms      │ 2.469 ms      │ 2.641 ms      │ 100     │ 100
   │  ├─ 100000             31.76 ms      │ 51.85 ms      │ 33.6 ms       │ 33.87 ms      │ 100     │ 100
   │  ╰─ 500000             177.1 ms      │ 200.2 ms      │ 188.3 ms      │ 188.3 ms      │ 100     │ 100
   ├─ bench_delete                        │               │               │               │         │
   │  ├─ 10000              1.827 ms      │ 8.458 ms      │ 1.915 ms      │ 2.131 ms      │ 100     │ 100
   │  ├─ 100000             38.49 ms      │ 71.4 ms       │ 46.03 ms      │ 46.47 ms      │ 100     │ 100
   │  ╰─ 500000             553.4 ms      │ 588.9 ms      │ 566.2 ms      │ 567.8 ms      │ 100     │ 100
   ├─ bench_insert                        │               │               │               │         │
   │  ├─ 10000              770.4 µs      │ 2.545 ms      │ 828.2 µs      │ 964.1 µs      │ 100     │ 100
   │  ├─ 100000             15.95 ms      │ 67.89 ms      │ 19.53 ms      │ 20.12 ms      │ 100     │ 100
   │  ╰─ 500000             340.3 ms      │ 374.4 ms      │ 350.4 ms      │ 350.9 ms      │ 100     │ 100
   ╰─ bench_query                         │               │               │               │         │
      ├─ 10000              1.664 ms      │ 4.792 ms      │ 1.784 ms      │ 1.984 ms      │ 100     │ 100
      ├─ 100000             375.3 ms      │ 553.9 ms      │ 410.5 ms      │ 417.7 ms      │ 100     │ 100
      ╰─ 500000             35.89 s       │ 43.07 s       │ 39.25 s       │ 39.14 s       │ 100     │ 100
```
¹ Creates the same graph as 'bench_insert' but uses a pre-populated adj map.

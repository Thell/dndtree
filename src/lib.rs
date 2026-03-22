#![deny(missing_docs)]
// #![forbid(unsafe_code)]

//! Implementation of the ID-Tree data structure from:
//! *“Constant-time Connectivity Querying in Dynamic Graphs”* (ACM, 2024).

mod dndtree;
pub use crate::dndtree::DNDTree;

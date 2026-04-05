# Variants

The full reference C++ implementation has buffered tree operations in-place
which the paper utilizes for temporal capabilities. The Rust implementations
do not have this capability. A part of the buffered operations includes dedup
of tree operations which the Rust implementations also do the remaining
overhead of the buffered operations is minor but measurable.

## C++
CPPDNDTree => Reference implementation accessed via ffi

## Rust
IDTree => Dedicated ID-Tree only build
RcDNDTree => DSU implemented as Rc based doubly linked list
RcDNDTree-NoDSU => Same as previous without DSU enabled
LstDNDTree- => DSU implemented as array back doubly linked list
LstDNDTree-NoDSU => Same as previous without DSU enabled

# Benches

The expensive DSU maintenance operations are avoided by the ID-Tree but it pays
by having to traverse the spanning tree for each tree operation.


## road-usroads-48.mtx

This is a medium sized (126k nodes) graph with average degree 2.

                         | CPPDNDTree |  IDTree    | RcDNDTree-NoDSU | RcDNDTree  | LstDNDTree-NoDSU | LstDNDTree
Result Type              | Mean (ns)  | Mean (ns)  | Mean (ns)       | Mean (ns)  | Mean (ns)        | Mean (ns)  
-------------------------------------------------------------------------------------------------------------------
--- INSERTION ---                                                                                                 
Non-Tree Edge            | 2713.17    | 2518.20    | 3459.13         | 2913.39    | 2486.33          | 1791.31    
Tree Edge                | 507.72     | 238.48     | 347.35          | 605.74     | 236.99           | 264.51     
Non-Tree Reroot          | 815.14     | 364.98     | 554.59          | 1236.58    | 359.28           | 431.91     
Tree Reroot              | 457.72     | 127.40     | 198.64          | 770.62     | 122.54           | 196.55     
-------------------------------------------------------------------------------------------------------------------
--- QUERY (COLD) ---                                                                                              
Disconnected             | 147.94     | 1353.91    | 3427.28         | 444.86     | 1010.35          | 74.35      
Connected                | 107.66     | 1196.67    | 5538.97         | 426.97     | 883.33           | 36.79      
-------------------------------------------------------------------------------------------------------------------
--- QUERY (WARM) ---                                                                                              
Disconnected             | 40.64      | 561.00     | 2311.87         | 39.25      | 441.66           | 29.32      
Connected                | 35.26      | 608.33     | 4198.86         | 33.94      | 473.33           | 34.12      
-------------------------------------------------------------------------------------------------------------------
--- DELETION ---                                                                                                  
Non-Tree Edge            | 241.40     | 78.98      | 106.98          | 118.26     | 66.46            | 78.58      
Tree Edge (Split)        | 5631.15    | 1526.80    | 4398.75         | 4779.68    | 1381.34          | 1486.56    
Tree Edge (Replaced)     | 998.13     | 146.10     | 420.45          | 1495.75    | 121.83           | 263.88     

## bdo_exploration_graph.mtx

This is a small planar graph (~1k nodes) with average 2.6 degrees.

                         | CPPDNDTree |  IDTree    | RcDNDTree-NoDSU | RcDNDTree  | LstDNDTree-NoDSU | LstDNDTree
Result Type              | Mean (ns)  | Mean (ns)  | Mean (ns)       | Mean (ns)  | Mean (ns)        | Mean (ns)  
------------------------------------------------------------------------------------------------------------------
--- INSERTION ---                                                                                           
Non-Tree Edge            | 265.09     | 146.96     | 184.23          | 265.09     | 144.38           | 126.64      
Tree Edge                | 148.56     | 51.84      | 89.67           | 148.56     | 53.04            | 58.15       
Non-Tree Reroot          | 326.74     | 167.72     | 238.46          | 326.74     | 158.94           | 150.34      
Tree Reroot              | 161.01     | 53.51      | 131.02          | 161.01     | 53.75            | 66.21       
-------------------------------------------------------------------------------------------------------------------
--- QUERY (COLD) ---                                                                                               
Disconnected             | 35.27      | 41.50      | 42.97           | 35.27      | 39.37            | 30.70       
Connected                | 36.39      | 47.67      | 52.26           | 36.39      | 43.80            | 32.19       
-------------------------------------------------------------------------------------------------------------------
--- QUERY (WARM) ---                                                                                               
Disconnected             | 31.81      | 41.08      | 29.88           | 31.81      | 37.65            | 29.25       
Connected                | 31.97      | 47.65      | 29.77           | 31.97      | 41.48            | 29.06       
-------------------------------------------------------------------------------------------------------------------
--- DELETION ---                                                                                                   
Non-Tree Edge            | 96.66      | 39.85      | 39.94           | 96.66      | 38.52            | 38.72       
Tree Edge (Split)        | 547.90     | 176.71     | 217.77          | 547.90     | 169.48           | 177.79      
Tree Edge (Replaced)     | 217.50     | 57.51      | 155.97          | 217.50     | 57.39            | 117.96      

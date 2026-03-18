Algorithm 1: ID-Insert
 Input: a new edge (𝑢, 𝑣) and the ID-Tree index
 Output: the updated ID-Tree
 1 𝑟𝑜𝑜𝑡𝑢 ← compute the root of 𝑢;
 2 𝑟𝑜𝑜𝑡𝑣 ← compute the root of 𝑣;
 /* non-tree edge insertion */
 3 if 𝑟𝑜𝑜𝑡𝑢 = 𝑟𝑜𝑜𝑡𝑣 then
 4   if 𝑑𝑒𝑝𝑡ℎ(𝑢) < 𝑑𝑒𝑝𝑡ℎ(𝑣) then swap(𝑢, 𝑣);
 5   if 𝑑𝑒𝑝𝑡ℎ(𝑢) − 𝑑𝑒𝑝𝑡ℎ(𝑣) ≤ 1 then return;
     /* reduce tree deviation */
 6   𝑤 ← 𝑢;
 7   for 1 ≤ 𝑖 < (𝑑𝑒𝑝𝑡ℎ(𝑢) − 𝑑𝑒𝑝𝑡ℎ(𝑣)) / 2 do
 8     𝑤 ← 𝑝𝑎𝑟𝑒𝑛𝑡(𝑤);
 9   Unlink(𝑤);
 10  Link(ReRoot(𝑢), 𝑣, 𝑟𝑜𝑜𝑡𝑣 );
 11  return;
 /* tree edge insertion */
 12 if 𝑠𝑡_𝑠𝑖𝑧𝑒(𝑟𝑜𝑜𝑡𝑢) > 𝑠𝑡_𝑠𝑖𝑧𝑒(𝑟𝑜𝑜𝑡𝑣) then
 13 swap(𝑢, 𝑣);
 14 swap(𝑟𝑜𝑜𝑡𝑢, 𝑟𝑜𝑜𝑡𝑣);
 15 Link(ReRoot(𝑢), 𝑣, 𝑟𝑜𝑜𝑡𝑣);


Most processes are the same as that of D-Tree. We first conduct a connectivity
query to identify if two vertices are in the same tree. Lines 3–11 apply the
BFS heuristic for non-tree edge insertion. When the depth gap between 𝑢 and 𝑣
is over 1, a major difference from D-Tree is about the strategy of BFS heuristic in
Line 7. D-Tree uses the threshold 𝑑𝑒𝑝𝑡ℎ(𝑢) − 𝑑𝑒𝑝𝑡ℎ(𝑣) − 2 instead of 𝑑𝑒𝑝𝑡ℎ (𝑢) −𝑑𝑒𝑝𝑡ℎ(𝑣)2 . To reduce
the average depth, we add half vertices from 𝑢 to its ancestor with the same depth of 𝑣 to the

- ReRoot(𝑢) rotates the tree and makes 𝑢 as the new root. It updates the parent-child relationship
  and the subtree size attribute from 𝑢 to the original root. The time complexity of ReRoot() is 𝑂(𝑑𝑒𝑝𝑡ℎ(𝑢)).

ReRoot from Algorithm 1 of Dynamic Spanning Trees for Connectivity Queries on Fully-dynamic Undirected Graphs (Extended Version):
Algorithm: reroot(nw)
input : tree node nw of D-tree with the root r
output : nw, new root of the rerooted D-tree
1 ch = nw; cur = nw.parent; nw.parent = NULL;
2 while cur, NULL do
3   g = cur.parent
4   cur.parent = ch
5   remove ch from cur.children
6   add cur to ch.children
7   ch = cur; cur = g;
8 while ch.parent, NULL do
9   ch.size = ch.size - ch.parent.size
10  ch.parent.size = ch.parent.size + ch.size
11  ch = ch.parent
12 return uw

- Link(𝑢, 𝑣, 𝑟𝑜𝑜𝑡𝑣) adds a tree 𝑇𝑢 rooted in 𝑢 to the children of 𝑣. 𝑟𝑜𝑜𝑡𝑣 is the root of 𝑣. Given that
  the subtree size of 𝑣 is changed, it updates the subtree size for each vertex from 𝑣 to the root.
  We apply the centroid heuristic by recording the first vertex with a subtree size larger than 𝑠𝑡_𝑠𝑖𝑧𝑒(𝑟𝑜𝑜𝑡𝑣)/2.
  If such a vertex is found, we reroot the tree, and the operator returns the new root.
  The time complexity of Link() is 𝑂(𝑑𝑒𝑝𝑡ℎ(𝑣)).

Link from Algorithm 6 of Dynamic Spanning Trees for Connectivity Queries on Fully-dynamic Undirected Graphs (Extended Version):
Algorithm: link(𝑛𝑢, 𝑟𝑢, 𝑛𝑣)
input : a node 𝑛𝑢 in D-tree 𝐷 with the root 𝑟𝑢, the root 𝑛𝑣 of a D-tree currently not connected to 𝐷 via a tree edge
output : merged D-tree with new tree edge (𝑛𝑢, 𝑛𝑣)
1 add 𝑛𝑣 to 𝑛𝑢.𝑐ℎ𝑖𝑙𝑑𝑟𝑒𝑛
2 𝑛𝑣.𝑝𝑎𝑟𝑒𝑛𝑡 = 𝑛𝑢
3 𝑚 = 𝑁𝑢𝑙𝑙; // new centroid
4 𝑖 = 𝑛𝑢
5 while 𝑖 ≠ 𝑁𝑢𝑙𝑙 do
6   𝑖.𝑠𝑖𝑧𝑒 = 𝑖.𝑠𝑖𝑧𝑒 + 𝑛𝑣.𝑠𝑖𝑧𝑒
7   if 𝑖.𝑠𝑖𝑧𝑒> (𝑟𝑢.𝑠𝑖𝑧𝑒 + 𝑛𝑣.𝑠𝑖𝑧𝑒)/2 and 𝑚 == 𝑁𝑢𝑙𝑙 then 𝑚 = 𝑖;
8   𝑖 = 𝑖.𝑝𝑎𝑟𝑒𝑛𝑡
9 if 𝑚 ≠ 𝑁𝑢𝑙𝑙 and 𝑚 ≠ 𝑟𝑢 then 𝑟𝑢

- Unlink(𝑢) disconnect the subtree of 𝑢 from the original tree.
  All ancestors of 𝑢 are scanned to update the subtree size.
  The time complexity of Unlink() is 𝑂 (𝑑𝑒𝑝𝑡ℎ(𝑢)).

UnLink from Algorithm 7 of Dynamic Spanning Trees for Connectivity Queries on Fully-dynamic Undirected Graphs (Extended Version):
Algorithm: unlink(𝑢)
input : a non-root node 𝑢 in D-tree 𝐷
output : two D-trees, not connected via tree edges
1 𝑖 = 𝑢
2 while 𝑖.𝑝𝑎𝑟𝑒𝑛𝑡 ≠ 𝑁𝑢𝑙𝑙 do
3   𝑖 = 𝑖.𝑝𝑎𝑟𝑒𝑛𝑡
4   𝑖.𝑠𝑖𝑧𝑒 = 𝑖.𝑠𝑖𝑧𝑒 − 𝑢.𝑠𝑖𝑧𝑒
5 remove 𝑢 from 𝑢.𝑝𝑎𝑟𝑒𝑛𝑡.𝑐ℎ𝑖𝑙𝑑𝑟𝑒𝑛
6 𝑢.𝑝𝑎𝑟𝑒𝑛𝑡 = 𝑁𝑢𝑙𝑙
7 return (𝑢, 𝑖)


Algorithm 2: ID-Delete
Input: an existing edge (𝑢, 𝑣) and the ID-Tree
Output: the updated ID-Tree
1 if 𝑝𝑎𝑟𝑒𝑛𝑡(𝑢) ≠ 𝑣 ∧ 𝑝𝑎𝑟𝑒𝑛𝑡(𝑣) ≠ 𝑢 then return;
2 if 𝑝𝑎𝑟𝑒𝑛𝑡(𝑣) = 𝑢 then swap(𝑢, 𝑣);
3 𝑟𝑜𝑜𝑡𝑣 ← Unlink(𝑢);
/* reduce the worst-case time complexity of searching replacement edge in subtree */
4 if 𝑠𝑡_𝑠𝑖𝑧𝑒(𝑟𝑜𝑜𝑡𝑣) < 𝑠𝑡_𝑠𝑖𝑧𝑒(𝑢) then swap(𝑢, 𝑟𝑜𝑜𝑡𝑣);
/* search subtree rooted in 𝑢 */
5 𝑄 ← an empty queue, 𝑄.𝑝𝑢𝑠ℎ(𝑢);
6 𝑆 ← {𝑢};
/* 𝑆 maintains all visited vertices */
7 while 𝑄 ≠ ∅ do
8   𝑥 ← 𝑄.𝑝𝑜𝑝 ();
9   foreach 𝑦 ∈ 𝑁 (𝑥) do
10    if 𝑦 = 𝑝𝑎𝑟𝑒𝑛𝑡 (𝑥) then continue;
11    else if 𝑥 = 𝑝𝑎𝑟𝑒𝑛𝑡 (𝑦) then
12      𝑄.𝑝𝑢𝑠ℎ(𝑦);
13      𝑆 ← 𝑆 ∪ {𝑦};
14    else
15      𝑠𝑢𝑐𝑐 ← true;
16      foreach 𝑤 from 𝑦 to the root do
17      if 𝑤 ∈ 𝑆 then
18        𝑠𝑢𝑐𝑐 ← false;
19        break;
20      else
21        𝑆 ← 𝑆 ∪ {𝑤 };
22      if 𝑠𝑢𝑐𝑐 then
23        𝑟𝑜𝑜𝑡𝑣 ← Link(ReRoot(𝑥), 𝑦, 𝑟𝑜𝑜𝑡𝑣 );
24        return;

Algorithm 3: Disjoint-set operators
1 Procedure Find(𝑥)
2 if 𝑥.𝑝𝑎𝑟𝑒𝑛𝑡 ≠ 𝑥 then
3   𝑥.𝑝𝑎𝑟𝑒𝑛𝑡 ← Find(𝑥);
4   return 𝑥.𝑝𝑎𝑟𝑒𝑛𝑡;
5 return 𝑥;

6 Procedure Union(𝑥, 𝑦)
7 𝑥 ← Find(𝑥);
8 𝑦 ← Find(𝑦);
9 if 𝑥 = 𝑦 then return;
10 if 𝑥.𝑠𝑖𝑧𝑒 > 𝑦.𝑠𝑖𝑧𝑒 then swap(𝑥, 𝑦);
11 𝑥.𝑝𝑎𝑟𝑒𝑛𝑡 ← 𝑦;
12 𝑦.𝑠𝑖𝑧𝑒 ← 𝑥.𝑠𝑖𝑧𝑒 + 𝑦.𝑠𝑖𝑧𝑒;

Double Linked List
DSnode
- 𝑖𝑑 // id of the corresponding vertex;
- 𝑝𝑎𝑟𝑒𝑛𝑡 // pointer to the parent’s DSnode in 𝐷𝑆-tree;
- 𝑝𝑟𝑒 // previous pointer in the DLL of the parent’s children;
- 𝑛𝑒𝑥𝑡 // next pointer in the DLL of the parent’s children;
- 𝑐ℎ𝑖𝑙𝑑𝑟𝑒𝑛 // start position of the DDL of children.

Algorithm 4: DS-Tree operators
1 Procedure UnlinkDS(𝑢)
2 DSnode(𝑢).𝑝𝑟𝑒.𝑛𝑒𝑥𝑡 ← DSnode(𝑢).𝑛𝑒𝑥𝑡;
3 DSnode(𝑢).𝑛𝑒𝑥𝑡.𝑝𝑟𝑒 ← DSnode(𝑢).𝑝𝑟𝑒;
4 DSnode(𝑢).𝑝𝑎𝑟𝑒𝑛𝑡 ← DSnode(𝑢);
5 DSnode(𝑢).𝑝𝑟𝑒 ← Null;
6 DSnode(𝑢).𝑛𝑒𝑥𝑡 ← Null;

7 Procedure LinkDS(𝑢, 𝑣)
/* union without find and comparing size */
/* the input satisfies 𝑠𝑡_𝑠𝑖𝑧𝑒 (𝑢) ≤ 𝑠𝑡_𝑠𝑖𝑧𝑒 (𝑣) */
/* union two DS-Trees */
8 DSnode(𝑢).𝑝𝑎𝑟𝑒𝑛𝑡 ← DSnode(𝑣);
/* add 𝑢 to the new DLL */
9 DSnode(𝑢).𝑝𝑟𝑒 ← DSnode(𝑣).𝑐ℎ𝑖𝑙𝑑𝑟𝑒𝑛;
10 DSnode(𝑢).𝑛𝑒𝑥𝑡 ← DSnode(𝑣).𝑐ℎ𝑖𝑙𝑑𝑟𝑒𝑛.𝑛𝑒𝑥𝑡;
11 DSnode(𝑢).𝑝𝑟𝑒.𝑛𝑒𝑥𝑡 ← DSnode(𝑢);
12 DSnode(𝑢).𝑛𝑒𝑥𝑡 .𝑝𝑟𝑒 ← DSnode(𝑢);

13 Procedure FindDS(𝑢)
14 if DSnode(𝑢).𝑝𝑎𝑟𝑒𝑛𝑡 ≠ DSnode(𝑢) then
15   𝑟𝑜𝑜𝑡 ← FindDS(DSnode(𝑢).𝑝𝑎𝑟𝑒𝑛𝑡.𝑖𝑑);
16   UnlinkDS(𝑢);
17   LinkDS(𝑢, 𝑟𝑜𝑜𝑡);
18   return 𝑟𝑜𝑜𝑡;
19 return 𝑢;

20 Procedure Isolate(𝑢)
/* assign children of 𝑢 to the root */
21 𝑟𝑜𝑜𝑡𝑢 ← FindDS(𝑢);
22 UnlinkDS(𝑢);
23 foreach child 𝑤 of 𝑢 in 𝐷𝑆-Tree do
24   UnlinkDS(𝑤);
25   LinkDS(𝑤, 𝑟𝑜𝑜𝑡𝑢 );

26 Procedure ReRootDS(𝑢)
27 𝑟𝑜𝑜𝑡𝑢 ← FindDS(𝑢);
28 swap(DSnode(𝑢), DSnode(𝑟𝑜𝑜𝑡𝑢));
29 DSnode(𝑢).𝑖𝑑 ← 𝑢;
30 DSnode(𝑟𝑜𝑜𝑡𝑢).𝑖𝑑 ← 𝑟𝑜𝑜𝑡𝑢;

Algorithm 5: DND-Insert
Input: an existing edge (𝑢, 𝑣) and the DND-Trees index
Output: the updated DND-Trees
1 𝑟𝑜𝑜𝑡𝑢 ← FindDS(𝑢);
2 𝑟𝑜𝑜𝑡𝑣 ← FindDS(𝑣);
  /* 3 Lines 3–14 of Algorithm 1; */
  /* non-tree edge insertion */
  3 if 𝑟𝑜𝑜𝑡𝑢 = 𝑟𝑜𝑜𝑡𝑣 then
  4   if 𝑑𝑒𝑝𝑡ℎ(𝑢) < 𝑑𝑒𝑝𝑡ℎ(𝑣) then swap(𝑢, 𝑣);
  5   if 𝑑𝑒𝑝𝑡ℎ(𝑢) − 𝑑𝑒𝑝𝑡ℎ(𝑣) ≤ 1 then return;
      /* reduce tree deviation */
  6   𝑤 ← 𝑢;
  7   for 1 ≤ 𝑖 < (𝑑𝑒𝑝𝑡ℎ(𝑢) − 𝑑𝑒𝑝𝑡ℎ(𝑣)) / 2 do
  8     𝑤 ← 𝑝𝑎𝑟𝑒𝑛𝑡(𝑤);
  9   Unlink(𝑤);
  10  Link(ReRoot(𝑢), 𝑣, 𝑟𝑜𝑜𝑡𝑣 );
  11  return;
  /* tree edge insertion */
  12 if 𝑠𝑡_𝑠𝑖𝑧𝑒(𝑟𝑜𝑜𝑡𝑢) > 𝑠𝑡_𝑠𝑖𝑧𝑒(𝑟𝑜𝑜𝑡𝑣) then
  13   swap(𝑢, 𝑣);
  14   swap(𝑟𝑜𝑜𝑡𝑢, 𝑟𝑜𝑜𝑡𝑣);
4 ReRoot(𝑢);
5 LinkDS(𝑟𝑜𝑜𝑡𝑢, 𝑟𝑜𝑜𝑡𝑣);
6 Link(𝑢, 𝑣, 𝑟𝑜𝑜𝑡𝑣);

Algorithm 6: DND-Delete
Input: an existing edge (𝑢, 𝑣) and the DND-Trees index
Output: the updated DND-Trees
1 (𝑢, 𝑟𝑜𝑜𝑡𝑣, 𝑠𝑢𝑐𝑐, 𝑆) ← ID-Delete(𝑢, 𝑣);
2 ReRootDS(𝑟𝑜𝑜𝑡𝑣);
3 if 𝑠𝑢𝑐𝑐 then return;
/* 𝑢 is the root of the smaller ID-Tree */
4 Isolate(𝑢);
5 𝑆 ← 𝑆 \ {𝑢};
6 foreach 𝑤 ∈ 𝑆 do
7   Isolate(𝑤);
8   LinkDS(𝑤, 𝑢);

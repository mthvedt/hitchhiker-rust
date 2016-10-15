The Thunderhead data store is a persistent hitchhiker B-tree, supporting the following use cases:
- Fast snapshots. (Actually, saving a snapshot is faster than not saving one.)
- Fast snapshot diffs.
- Range queries and locking.
- Within-tree references.
- Subtree views. (Is this needed?)
- Persistence.

Yak shave
=========

- Decide if and why we needed MemNodes! Advantage: it's already written, container types are easily swapped.

- Impl a trait for mutable Nodes with common pointer types. We don't need to create a node trait,
but we need this because:
- - We need nice, small objects for MemNode return values, and objects can't have associated types.
- - We might have multiple MemNode types with the same data layout (e.g. leaf and non-leaf).
- - - For the above, consider having a dynamically sized child array instead.

- Implement MemNode, a BTree node with tweakable parameters.
(Why? We wanted a BTree memory node with multiple formats... Any reason for that? Anyone? Bueller?
The idea was that committed and uncommitted in-memory nodes would have different formats but share the memnode layout...
This is probably wrong!

It all started with the observation that for cold nodes, we want to store node info in the node address.
For hot nodes we have no particular desire. Uncommitted nodes can address other uncommitted nodes or cold nodes.
Transient nodes can address any one of the three.)

- Impl a node trait. (Optional)
- Implement FatNodeRef, give it ALL THE POWAH of a node. How? Through the Operations pattern, maybe.
Or just a switch enum... there's really only a few kinds of nodes.
- - We will probably need the Operations pattern anyway.
- Impl a node header with the snapshot information we need. Node pointers have the node header stored on their person.

Master TODO list
================

- Snapshot diffs.
- Hitchhiker model?
- Merge based commit OR concurrent writer commit. Or something. Pushdown rollbacks?
- - Option: Merge commit
- - - Advantage: Commit is just about destorying memory.
- - - Disadvantage: Merges can become nontrivial.
- - - Disadvantage: Need to figure out when to write information. For pushdown rollbacks this happens automatically.
- - Option: Pushdown rollbacks
- - - TODO: advantages and disadvantages.
- - - Advantage: don't need to figure out hitchhiker model first.
- Rudimentary locks.
- Within tree references.
- Persistence.
- Event model.

Design
======

- The general structure is that of a B+ tree, where data is stored in leaf nodes.
- Large data chunks are stored off-tree, as in PostgreSQL TOAST.
- There is a per-node journal (Hitchhiker Tree).
- Keys are compressed according to shared prefixes, including the prefixes of parent nodes.
- Transactions are conducted in-memory on separate copies and merged in. Large transactions may be spilled.


Nodes
=====

Nodes are defined with the Node trait.
Frequently Nodes are accessed through Handles. A Handle is a handle to a hot (viz. in-memory) node.
The existence of Handles can pin resources.

Nodes may be transient or persistent. Transient nodes are modifiable and may not be cloned.
Persistent nodes may not be modified.
Note that persistent and transient nodes may share underlying data.

TODO: we need to decide the story for commits.

The Thunderhead data store is a persistent hitchhiker B-tree, supporting the following use cases:
- Fast snapshots. (Actually, saving a snapshot is faster than not saving one.)
- Fast snapshot diffs.
- Range queries and locking.
- Within-tree references.
- Subtree views. (Is this needed?)
- Persistence.

Yak shave
=========

Need BucketRefs.

Master TODO list
================

- Cursors
- Hitchhiker model?

(In any order)
- Commit model.
- Persistence.
- Futures model.
- Rudimentary locks.
- Within tree references.

Commit model Qs
===============
- Merge based commit OR concurrent writer commit. Or something. Pushdown rollbacks?
- - Option: Merge commit
- - - Advantage: Commit is just about destorying memory.
- - - Disadvantage: Merges can become nontrivial.
- - - Disadvantage: Need to figure out when to write information. For pushdown rollbacks this happens automatically.
- - Option: Pushdown rollbacks
- - - TODO: advantages and disadvantages.
- - - Advantage: don't need to figure out hitchhiker model first.

Hitchhiker design
=================

'Base tree' of 256 nodes as follows:
Level 0 indices, an array of 256
Level 1 indices, key + inline value or value pointer + child pointer (if applicable)

'Hitchhiker overlay' as follows:
BTree node in same format as base tree...

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

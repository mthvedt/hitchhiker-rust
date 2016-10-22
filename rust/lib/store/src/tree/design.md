The Thunderhead data store is a persistent hitchhiker B-tree, supporting the following use cases:
- Fast snapshots. (Actually, saving a snapshot is faster than not saving one.)
- Fast snapshot diffs.
- Range queries and locking.
- Within-tree references.
- Subtree views. (Is this needed?)
- Persistence.

Yak shave
=========

Datum is totally unnecessary.
Keys should be Borrow<[u8]> for all cases except insert. Insert keys, and all values, should have some
sort of associated transaction thing, but that's not important right now. Is it?

In fact, key might be ENTIRELY obsolete. We can later implement the ability to pin zero-copy
buffers to transactions, but that's not important right now.

TODO: determine if key is entirely obsolete! When is it a good idea?

Master TODO list
================

- Code cleanup.
- Futures model?

A disk model should provide futures for getting and saving. Need to do some thinking about this.
It needs to be external.

- Fixup interior node polymorphism.
- Transaction and alloc model?

A transaction allows us to alloc arbitrary bytes, as well as arena-alloc various typed things.
Publically, we can alloc space for keys and values to be inserted into the tree without copying.
The returned 'AllocPointers' maybe from a transaction arena or may simply be boxes or Rcs.

(In any order)
- Hitchhiker nodes
- Commit model.
- Persistence.
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
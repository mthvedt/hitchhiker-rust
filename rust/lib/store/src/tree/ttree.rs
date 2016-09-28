/*
Note: We always copy the key structure. SSDs begin to slow down at 32-64k writes, so we might as well.
But how to handle in-memory modifications?

Design sketch: Flushing dirty pages
We track the size (in bytes) of all entries we need to flush.
Then we do the following:

- If all entries belong to one child, we flush that node as a 'dirty parent head' node.
- If entries are lower than the flush threshold, we write out an ordinary head node
(a dirty parent node if we are not head).
- Otherwise, we flush the dirtiest child and repeat.
-- Additional optimization: we look for a child we can flush into the same disk page.

Design sketch: GC

Each head knows two things: the refer value of the previous generation,
and the refer value of the current generation. The first is always decreasing.
Whenever the prev-gen-refer drops past a certain threshold, a GC is triggered...

but this doesn't solve the problem of LIVE data...

Idea two:
Each snapshot has a refer value. We always know the space difference between two snapshots.
Can we calculate the snapshots' refer deltas?

For any snapshot, we know data dropped and data added from the oldest.
If we have s1, s2, s3...
s3's add/drop wrt s1...

s2 drops X2 data prior to s2, adds Y2 data
s3 drops X3 data prior to s3, adds Y3 data
if we delete s2, then s3 drops X2 + X3 data and adds Y2 + Y3 data???

Idea: recursive tree:
Node stats (for HEAD + all snapshots) are stored in a special key 00. More node stats in 01...
und so weiter. We use this to track 'freeable' space. If the freeable space in a page
exceeds a certain amount, that page is eligible for GC.

Simpler idea: recursive tree with tracing GC
*/

// Tracks DB stats.
enum TopInfo {

}

trait NodeHeader {

}

trait HotNode {
	fn prefix(&self) -> PrefixKey;
	fn find(&self, k: PartialKey) -> Option<NodeHeaderHandle>;
}

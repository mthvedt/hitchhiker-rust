struct NodeAddress {

}

struct HitchhikerChildPointer {
	addr: NodeAddress,
}

struct HitchhikerHotNode {
	// A HitchhikerNode has the following format:
	// Key table
	// Keys
	// Child pointer table
	// Children
	// Value address table
	// Values

	// The on-disk format of the key table is such that in-memory operations can be stored inline.

	// TODO: implement this format.
}

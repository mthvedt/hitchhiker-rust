package com.thunderhead.core.kv.local;

/**
 * Created by mike on 7/22/16.
 */
public interface ReadResult {
    // TODO: prove that for a local transactional lattice, we can totally order it such that
    // every kv version can be associated with a snapshot range that forms the same transactional lattice.
    SnapshotRange getRange();
}

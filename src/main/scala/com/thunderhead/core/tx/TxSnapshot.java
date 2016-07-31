package com.thunderhead.core.tx;

/**
 * Created by Mike on 7/10/16.
 */
public interface TxSnapshot {
    TxStamp getStamp();

    // TODO: implements SnapshotRange?

    // TODO: maybe a store is a snapshot -> store map...

    void commitAndClose();
}

package com.thunderhead.core.kv.local;

/**
 * Created by mike on 7/21/16.
 */
public interface SnapshotRange {
    Version getMinVersion();

    Version getMaxVersion();
}

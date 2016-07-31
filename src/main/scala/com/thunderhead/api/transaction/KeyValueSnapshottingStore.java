package com.thunderhead.api.transaction;

import com.sun.tools.javac.util.Context;

import java.util.NavigableMap;

/**
 * Created by mike on 7/22/16.
 * TODO: public or private? User API must be clean as possible.
 */
public interface KeyValueSnapshottingStore {
    KeyValueSnapshot openEphemeralSnapshot();

    NavigableMap<Counter, KeyValueSnapshot> getBlockingSnapshotMap();
}

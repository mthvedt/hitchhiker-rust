package com.thunderhead.core.kv.combo;

import com.thunderhead.api.Datum;
import com.thunderhead.core.kv.local.SnapshotRange;

/**
 * Created by mike on 7/21/16.
 */
public interface ComboSnapshot {
    SnapshotRange getSnapshotFor(Datum key);

    // TODO: for ranges
//    ComboSnapshot getSubSnapshot(Datum rangeBegin, Datum rangeEnd);
}

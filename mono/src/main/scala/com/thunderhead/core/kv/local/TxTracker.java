package com.thunderhead.core.kv.local;

import com.thunderhead.api.transaction.TransactionView;

/**
 * Created by mike on 7/21/16.
 */
public interface TxTracker {
    SnapshotRange getSnapshot();

    TransactionView getView();
}

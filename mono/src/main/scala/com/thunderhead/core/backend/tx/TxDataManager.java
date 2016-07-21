package com.thunderhead.core.backend.tx;

import com.thunderhead.core.tx.TxDatastore;
import com.thunderhead.core.tx.TxStamp;

/**
 * Created by Mike on 7/10/16.
 */
public interface TxDataManager {
    TxDatastore getDatastore();

    void acceptCommit(TxStamp stamp);
    
    void rollbackCommit(TxStamp stamp);
}

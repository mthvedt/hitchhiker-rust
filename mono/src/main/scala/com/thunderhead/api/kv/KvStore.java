package com.thunderhead.api.kv;

/**
 * Created by mike on 7/21/16.
 */
public interface KvStore {
    KvTransaction openTx();
}

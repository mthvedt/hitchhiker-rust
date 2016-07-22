package com.thunderhead.api.kv;

import com.thunderhead.api.Datum;

/**
 * Created by mike on 7/21/16.
 */
public interface KvSnapshot {
    Datum get(Datum key);
}

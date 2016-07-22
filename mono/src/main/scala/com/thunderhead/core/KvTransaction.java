package com.thunderhead.core;

import com.thunderhead.api.Datum;

/**
 * Created by Mike on 7/9/16.
 */
public interface KvTransaction {
    void lockRange(Datum range);

    Datum read(Datum key);

    void write(Datum key, Datum value);

    boolean commit();
}

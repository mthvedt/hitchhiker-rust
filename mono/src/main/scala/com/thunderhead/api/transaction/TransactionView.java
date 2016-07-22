package com.thunderhead.api.transaction;

import com.thunderhead.api.Datum;

/**
 * Created by mike on 7/21/16.
 */
public interface TransactionView {
    Datum get(Datum key) throws TransactionAbortedException;

    void put(Datum key, Datum value) throws TransactionAbortedException;

    void delete(Datum key) throws TransactionAbortedException;

    void close() throws TransactionAbortedException;
}

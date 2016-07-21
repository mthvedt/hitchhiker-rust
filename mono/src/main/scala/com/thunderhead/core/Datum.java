package com.thunderhead.core;

/**
 * Created by Mike on 7/3/16.
 */
public interface Datum extends Cloneable, Comparable<Datum> {
    int length();

    byte byteAt(int index);
}

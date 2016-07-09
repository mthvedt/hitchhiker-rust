package com.thunderhead.core;

/**
 * Created by Mike on 7/3/16.
 */
public interface DbObject extends Cloneable, Comparable<DbObject> {
    int length();

    byte byteAt(int index);
}

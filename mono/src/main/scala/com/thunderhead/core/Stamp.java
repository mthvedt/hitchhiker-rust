package com.thunderhead.core;

/**
 * Created by Mike on 7/3/16.
 */
// TODO: interface and impl class
public class Stamp extends AbstractByteString<Stamp> implements Cloneable, Comparable<Stamp> {
    public Stamp(byte[] bytes) {
        super(bytes);

        if (bytes.length != 16) {
            throw new IllegalArgumentException("bytes length must be 16");
        }
    }
}

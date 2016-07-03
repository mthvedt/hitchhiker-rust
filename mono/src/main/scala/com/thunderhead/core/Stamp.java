package com.thunderhead.core;

/**
 * Created by Mike on 7/3/16.
 */
public class Stamp {
    public final byte[] bytes;

    public Stamp(byte[] bytes) {
        if (bytes.length != 16) {
            throw new IllegalArgumentException("Stamp length must be 16");
        }

        this.bytes = bytes.clone();
    }
}

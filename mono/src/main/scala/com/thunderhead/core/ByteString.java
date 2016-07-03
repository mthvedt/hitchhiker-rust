package com.thunderhead.core;

import java.util.Arrays;

/**
 * Created by Mike on 7/3/16.
 */
public final class ByteString implements Cloneable, Comparable<ByteString> {
    final byte[] bytes;

    public ByteString(byte[] bytes) {
        if (bytes.length == 0) {
            throw new IllegalArgumentException("bytes length cannot be 0");
        }

        this.bytes = bytes.clone();
    }

    public byte[] getBytes() {
        return bytes.clone();
    }

    @SuppressWarnings("CloneDoesntCallSuperClone")
    @Override
    public ByteString clone() {
        return new ByteString(bytes);
    }

    @Override
    public int compareTo(ByteString o) {
        for (int i = 0; ; i++) {
            if (i >= bytes.length) {
                if (i >= o.bytes.length) {
                    return 0;
                } else {
                    return -1; // dictionary ordering
                }
            } else {
                if (i >= o.bytes.length) {
                    return 1; // dictionary ordering
                } else {
                    byte a = bytes[i];
                    byte b = o.bytes[i];
                    int r = (int)a - (int)b;

                    if (r != 0) {
                        return r;
                    }
                }
            }
        }
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) {
            return true;
        }
        if (o == null || getClass() != o.getClass()) {
            return false;
        }

        ByteString that = (ByteString)o;
        return Arrays.equals(bytes, that.bytes);
    }

    @Override
    public int hashCode() {
        return Arrays.hashCode(bytes);
    }
}

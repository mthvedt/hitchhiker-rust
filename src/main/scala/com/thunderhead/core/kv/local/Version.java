package com.thunderhead.core.kv.local;

/**
 * Created by mike on 7/21/16.
 */
public final class Version implements Comparable<Version> {
    private final int num;

    public Version(int num) {
        this.num = num;
    }

    @Override
    public int compareTo(Version v) {
        // Comparison loops around intentionally.
        return num - v.num;
    }

    @Override
    public boolean equals(Object o) {
        return this == o || o instanceof Version && num == ((Version)o).num;

    }

    @Override
    public int hashCode() {
        return num;
    }
}

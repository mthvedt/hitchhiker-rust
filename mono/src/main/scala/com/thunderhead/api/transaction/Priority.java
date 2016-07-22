package com.thunderhead.api.transaction;

/**
 * Created by mike on 7/21/16.
 */
public class Priority implements Comparable<Priority> {
    private final short priority;

    public Priority(short priority) {
        this.priority = priority;
    }

    public short getPriority() {
        return priority;
    }

    @Override
    public int compareTo(Priority o) {
        return (int)priority - (int)o.priority;
    }
}

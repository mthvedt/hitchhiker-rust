package com.thunderhead.api;

/**
 * Created by mike on 7/23/16.
 */
public enum Priority {
    BATCH_CPU,
    THROUGHPUT_CPU,
    THROUGHPUT_IO,
    LATENCY_CPU,
    LATENCY_IO,
    URGENT
}

package com.thunderhead.core.fabric;

/**
 * Created by mike on 7/30/16.
 */
public interface IncomingMessage<T> {
    T getPayload();

    NodeHandle getSenderNode();
}

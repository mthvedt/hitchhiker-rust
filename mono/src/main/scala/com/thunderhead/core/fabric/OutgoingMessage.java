package com.thunderhead.core.fabric;

/**
 * Created by mike on 7/30/16.
 */
public interface OutgoingMessage<T> {
    T getPayload();

    NodeHandle getReceiverNode();
}

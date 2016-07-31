package com.thunderhead.core.reactor;

import com.thunderhead.api.Datum;
import com.thunderhead.core.fabric.Codec;
import com.thunderhead.core.fabric.IncomingMessage;
import com.thunderhead.core.fabric.OutgoingMessage;

/**
 * Created by mike on 7/30/16.
 */
public interface Reactor {
    void run(Runnable r);

    <T> void yield(T t, TaskListener<T> continuation);

    void yieldError(TaskError err, TaskListener<?> continuation);

    // TODO: net interface class?
    <T> void netSend(Datum port, OutgoingMessage request, Codec<T> encoder);

    <T> void addNetListener(Datum port, TaskListener<IncomingMessage<T>> responder, Codec<T> decoder);

    boolean removeNetListener(Datum port);
}

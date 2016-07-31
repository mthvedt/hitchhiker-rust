package com.thunderhead.api.async;

/**
 * Created by mike on 7/22/16.
 */
public interface Request<T> {
    void executeWith(Reactor r, Responder<T> responder);
}

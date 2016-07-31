package com.thunderhead.api.async;

/**
 * Created by mike on 7/22/16.
 */
public interface Reactor {
    short logResponder(Responder<?> r);

    <T> T respond(short stamp, T value);
}

package com.thunderhead.core.reactor;

/**
 * Created by mike on 7/30/16.
 */
public interface TaskListener<T> {
    void taskFinished(T arg, Reactor r);
    void taskError(TaskError err, Reactor r);
}

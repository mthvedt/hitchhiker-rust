package com.thunderhead.core.engine;

import com.thunderhead.core.reactor.Reactor;
import com.thunderhead.core.reactor.TaskListener;

/**
 * Created by mike on 7/30/16.
 */
public interface TaskQueue extends TaskSource {
    void enqueue(Task t);

    int defer(TaskListener<?> listener);

    void complete(int taskNum, Object arg, Reactor reactor);
}

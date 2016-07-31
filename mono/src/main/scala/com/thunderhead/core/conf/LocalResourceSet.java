package com.thunderhead.core.conf;

import java.util.List;

/**
 * Created by mike on 7/28/16.
 *
 * TODO: some kind of general query/map system for system resources.
 * How do other libraries do it?
 *
 * The question we have to answer first is: what do local data processes consume?
 */
public interface LocalResourceSet {
    List<LocalResourceSet> divideBy(Class<? extends Resource> resourceClass);

    <T extends Resource> List<T> getResource(Class<T> resourceClass);
}

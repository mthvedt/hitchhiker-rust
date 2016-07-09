package com.thunderhead.core;

/**
 * Created by Mike on 7/3/16.
 */
public interface DataManager {
    State getKnownState();

    Stamp getLocalStamp();

    void mergeWith(State other) throws TdException;

    // TODO: what does data manager merging look like?
    // Think of what the fastest solution would be.

    // The primitive operation is... merging?

    // What does the start/restart scenario look like?
}

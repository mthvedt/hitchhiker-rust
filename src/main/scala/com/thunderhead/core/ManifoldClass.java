package com.thunderhead.core;

/**
 * Created by mike on 7/26/16.
 */
public enum ManifoldClass {
    /**
     * The entire deployment. Not really supported at this time.
     */
    MULTISITE,

    /**
     * A deployment site, with low-latency inter-machine communication.
     */
    SITE,
    MACHINE,
    THREAD
}

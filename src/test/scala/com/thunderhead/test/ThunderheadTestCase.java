package com.thunderhead.test;

import com.thunderhead.core.conf.LocalEnvironment;
import org.springframework.cglib.core.Local;

import javax.inject.Inject;

/**
 * Created by mike on 8/2/16.
 */
public abstract class ThunderheadTestCase {
    private LocalEnvironment environment;

    @Inject
    protected void setLocalEnvironment(LocalEnvironment environment) {
        this.environment = environment;
    }

    protected LocalEnvironment getLocalEnvironment() {
        return environment;
    }
}

package com.thunderhead.test;

import com.thunderhead.core.conf.LocalEnvironment;
import org.junit.runner.RunWith;
import org.springframework.cglib.core.Local;
import org.springframework.test.context.TestExecutionListeners;
import org.springframework.test.context.junit4.SpringJUnit4ClassRunner;
import org.springframework.test.context.support.DependencyInjectionTestExecutionListener;
import org.springframework.test.context.support.DirtiesContextTestExecutionListener;

import javax.inject.Inject;

/**
 * Created by mike on 8/2/16.
 *
 * We should put as much Spring-specific stuff in this class as possible.
 * A design goal here is to separate the test environment provider from
 * the provider mechanism: the testing code shouldn't know how the
 * context was created. This separation is not completely clean because
 * we use Spring annotations specify contexts, but it's
 * the best we can do for now.
 */
@RunWith(SpringJUnit4ClassRunner.class)
@TestExecutionListeners(listeners = {
    DependencyInjectionTestExecutionListener.class,DirtiesContextTestExecutionListener.class
})
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

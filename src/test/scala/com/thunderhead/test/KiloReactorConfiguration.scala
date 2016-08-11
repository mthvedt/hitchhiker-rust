package com.thunderhead.test

import com.thunderhead.core.conf.LocalEnvironment
import com.thunderhead.mock.fabric.SingleThreadMockFabric
import org.springframework.context.annotation.{Bean, Configuration}

/**
  * Created by Mike on 7/31/16.
  */
@Configuration
class KiloReactorConfiguration extends EnvironmentProvider {
  // TODO: be able to simulate a fabric of environments.
  override def localEnvironment(): LocalEnvironment = new SingleThreadMockFabric(1024)
}

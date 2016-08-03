package com.thunderhead.test

import com.thunderhead.core.conf.LocalEnvironment
import com.thunderhead.mock.fabric.SingleThreadMockFabric
import org.springframework.context.annotation.Configuration

/**
  * Created by Mike on 7/31/16.
  */
class KiloReactorConfiguration extends EnvironmentProvider {
  // TODO: be able to simulate a fabric of environments.
  override def getLocalEnvironment: LocalEnvironment = new SingleThreadMockFabric(1024)
}

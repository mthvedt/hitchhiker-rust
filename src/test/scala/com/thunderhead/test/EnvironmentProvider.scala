package com.thunderhead.test

import com.thunderhead.core.conf.LocalEnvironment
import org.springframework.context.annotation.{Bean, Configuration}

/**
  * Created by mike on 8/1/16.
  */
abstract class EnvironmentProvider {
  // TODO: be able to simulate a fabric of environments.
  // TODO: support local and multimachine distributed tests.
  @Bean
  def localEnvironment(): LocalEnvironment

//  protected def internalGetLocalEnvironment: LocalEnvironment
}

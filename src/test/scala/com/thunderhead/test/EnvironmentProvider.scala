package com.thunderhead.test

import com.thunderhead.core.conf.LocalEnvironment
import org.springframework.context.annotation.{Bean, Configuration}

/**
  * Created by mike on 8/1/16.
  *
  * A design goal here is to separate the test environment provider from
  * the provider mechanism: the testing code shouldn't know how the
  * context was created. This separation is not completely clean because
  * we use Spring annotations on tests to specify contexts, but it's
  * the best we can do for now.
  */
@Configuration
abstract class EnvironmentProvider {
  // TODO: be able to simulate a fabric of environments.
  // TODO: support local and multimachine distributed tests.
  @Bean
  def getLocalEnvironment: LocalEnvironment
}

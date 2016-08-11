package com.thunderhead.core

import com.thunderhead.test.{KiloReactorConfiguration, ThunderheadTestCase}
import org.junit.Test
import org.springframework.test.context.ContextConfiguration

/**
  * Created by Mike on 7/31/16.
  */
@ContextConfiguration(classes = Array(
  classOf[KiloReactorConfiguration]
))
class SimpleReactorIntegrationTest extends ThunderheadTestCase {

  @Test
  def messagePassingTest(): Unit = {
    getLocalEnvironment().getClass()
    // For each reactor, pass a message 1000 times.
//    getLocalEnvironment().forEachReactor(new ReactorStarter {
//
//    })
  }
}

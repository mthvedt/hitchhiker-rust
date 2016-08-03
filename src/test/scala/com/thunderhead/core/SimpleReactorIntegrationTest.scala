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
  def testNothing(): Unit = {}
}

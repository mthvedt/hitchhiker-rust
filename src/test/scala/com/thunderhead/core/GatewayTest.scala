package com.thunderhead.core

import com.thunderhead.test.{KiloReactorConfiguration, ThunderheadTest}
import org.springframework.test.context.ContextConfiguration

/**
  * Created by Mike on 7/31/16.
  */
@ContextConfiguration(classes = {
  classOf[KiloReactorConfiguration]
})
class GatewayTest extends ThunderheadTest {
}

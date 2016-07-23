package com.thunderhead.core

import com.thunderhead.api.async.Reactor

/**
  * Created by mike on 7/22/16.
  */
abstract class Responder[T] {
  def continue(t: T, r: Reactor)
}

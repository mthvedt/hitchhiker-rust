package com.thunderhead.core

import com.thunderhead.api.async.Reactor

/**
  * Created by mike on 7/22/16.
  */
abstract class TaskListener[T] {
  def continue(t: T, r: Reactor)
  def handleError(err: Object, r: Reactor)
}

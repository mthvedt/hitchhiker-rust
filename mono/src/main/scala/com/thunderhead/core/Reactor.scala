package com.thunderhead.core

/**
  * Created by mike on 7/22/16.
  */
abstract class Reactor {
  def trampoline[T](t: T, responder: Responder[T])
}

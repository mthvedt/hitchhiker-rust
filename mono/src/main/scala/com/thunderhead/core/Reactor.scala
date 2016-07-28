package com.thunderhead.core

import com.thunderhead.core.fabric.{Codec, Message, NodeHandle}
import com.thunderhead.core.impl.ManifoldInfoImpl

/**
  * Created by mike on 7/22/16.
  */
trait Reactor {
  def trampoline[T](t: T, responder: TaskListener[T])
  def trampolineError[T](err: Object, responder: TaskListener[T])
  def netSend[T](request: Message, target: NodeHandle, responder: TaskListener[T], codec: Codec[T])
  // TODO ManifoldInfoImpl -> ManifoldInfo
  def manifoldInfo(): ManifoldInfoImpl
}

package com.thunderhead.core.fabric

/**
  * Created by mike on 7/25/16.
  */
trait MessagePacket[T] {
  def exists(): Boolean
  def message(): OutgoingMessage[T]
  def sender(): NodeHandle
  def taskId(): Int
}

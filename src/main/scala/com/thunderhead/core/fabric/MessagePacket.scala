package com.thunderhead.core.fabric

/**
  * Created by mike on 7/25/16.
  */
trait MessagePacket {
  def exists(): Boolean
  def message(): OutgoingMessage
  def sender(): NodeHandle
  def taskId(): Int
}

package com.thunderhead.core.fabric

/**
  * Created by mike on 7/25/16.
  */
trait Gateway {
  def send(obj: OutgoingMessage, taskId: Int, target: NodeHandle)
  def recv(): MessagePacket
}

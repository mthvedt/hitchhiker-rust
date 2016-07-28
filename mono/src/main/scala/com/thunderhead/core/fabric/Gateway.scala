package com.thunderhead.core.fabric

/**
  * Created by mike on 7/25/16.
  */
trait Gateway {
  def send(obj: Message, taskId: Int, target: NodeHandle)
  def recv(): MessagePacket
}

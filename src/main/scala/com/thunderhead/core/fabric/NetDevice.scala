package com.thunderhead.core.fabric

/**
  * Created by mike on 7/26/16.
  */
trait NetDevice {
  def send(): Unit
  def poll(): Unit
}

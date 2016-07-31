package com.thunderhead.core.fabric

/**
  * Created by mike on 7/26/16.
  */
trait DiskDevice {
  def sendIop(): Int
  def nextRetiredIop(): Either[Int, Exception]
}
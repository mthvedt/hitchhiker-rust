package com.thunderhead.core

/**
  * Created by mike on 7/25/16.
  */
// TODO: remove reactor method, ReactorManager => TaskSource?
trait ReactorManager {
  def reactor(): Reactor
  def nextTask(): Option[ReactorTask]
}

package com.thunderhead.mock.fabric

import com.thunderhead.core.{Reactor, ReactorManager, ReactorTask}

/**
  * Created by mike on 7/25/16.
  */
class ComboReactorManager(managers: Array[ReactorManager]) extends ReactorManager {
  var managerIndex: Int = 0

  // The flow should look like this:
  // Start a bunch of reactors.
  // Bootstrap.
  // On shutdown, shut down (derp).
  // TODO think about this method
  override def reactor(): Reactor = throw new IllegalStateException()

  override def nextTask(): Option[ReactorTask] = {
    // TODO consider a more fair allocation, based on recorded execution time

    var task: Option[ReactorTask] = None
    val previousManagerIndex = managerIndex

    do {
      task = managers(managerIndex).nextTask()
      managerIndex += 1
      if (managerIndex >= managers.length) {
        managerIndex = 0
      }
    } while (task.isEmpty && previousManagerIndex != managerIndex)

    task
  }
}

package com.thunderhead.core.engine.mock

import java.util.concurrent.atomic.AtomicBoolean

import com.thunderhead.core.engine.TaskSource

/**
  * Created by mike on 7/25/16.
  */
class TaskThread(source: TaskSource) {
  val interrupted = new AtomicBoolean(false)
  val running = new AtomicBoolean(false)

  private def runLoop() = {
    var shouldRun = !interrupted.get()

    while (shouldRun) {
      // Start of loop
      var didATask = false

      // Main body of loop
      source.dequeue() match {
        case null => Unit
        case task =>
          task.run()
          didATask = true
      }

      // End of loop
      if (!didATask) {
        // don't waste energy
        // TODO configurable sleep time
        Thread.sleep(1)
      }

      shouldRun = !interrupted.get()
    }
  }

  def start() = {
    var shouldStart = false

    running.synchronized {
      if (!running.get()) {
        running.set(true)
        interrupted.set(false)
        shouldStart = true
      }
    }

    if (shouldStart) {
      runLoop()
    }
  }

  def stop() = {
    interrupted.set(true)
  }
}

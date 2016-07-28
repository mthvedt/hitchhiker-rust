package com.thunderhead.mock.fabric

import java.util.concurrent.atomic.AtomicBoolean

import com.thunderhead.core.ReactorManager

/**
  * Created by mike on 7/25/16.
  */
class ReactorThread(r: ReactorManager) {
  val interrupted = new AtomicBoolean(false)
  val running = new AtomicBoolean(false)

  private def runLoop() = {
    var shouldRun = !interrupted.get()

    while (shouldRun) {
      // Start of loop
      var didATask = false

      // Main body of loop
      r.nextTask() match {
        case Some(task) =>
          task.execute()
          didATask = true
        case None => Unit
      }

      // End of loop
      if (!didATask) {
        // dont waste energy
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

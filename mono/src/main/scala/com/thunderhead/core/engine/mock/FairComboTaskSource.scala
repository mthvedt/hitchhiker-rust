package com.thunderhead.core.engine.mock

import com.thunderhead.core.engine.{Task, TaskQueue, TaskSource}

/**
  * Created by mike on 7/30/16.
  */
class FairComboTaskSource(qs: IndexedSeq[TaskQueue]) extends TaskSource {
  var qIndex = 0

  override def dequeue(): Task = {
    var task: Task = null
    val previousManagerIndex = qIndex

    do {
      task = qs(qIndex).dequeue()
      qIndex += 1
      if (qIndex >= qs.length) {
        qIndex = 0
      }
    } while (task != null && previousManagerIndex != qIndex)

    // TODO: make fair using timing in some way
    task
  }
}

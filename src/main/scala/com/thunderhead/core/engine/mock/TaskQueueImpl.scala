package com.thunderhead.core.engine.mock

import java.util.concurrent.ConcurrentLinkedQueue

import com.thunderhead.core.engine.{Task, TaskQueue}
import com.thunderhead.core.reactor.{Reactor, TaskListener}

import scala.collection.mutable

/**
  * Created by mike on 7/30/16.
  */
class TaskQueueImpl extends TaskQueue {
  val q: ConcurrentLinkedQueue[Task] = new ConcurrentLinkedQueue[Task]()

  val map: mutable.Map[Int, TaskListener[_]] = mutable.HashMap.empty

  var lowKey: Int = 0
  var highKey: Int = 1

  override def enqueue(t: Task): Unit = {
    q.add(t)
  }

  override def defer(listener: TaskListener[_]): Int = {
    if (lowKey == highKey) {
      // TODO do something better, but it's very unlikely this will happen
      throw new RuntimeException("overloaded")
    }

    val key = highKey
    map.put(key, listener)
    highKey += 1
    key
  }

  override def complete(taskNum: Int, arg: Any, r: Reactor): Unit = {
    val listener = map(taskNum)

    while (!map.contains(lowKey)) {
      lowKey += 1
    }

    enqueue(new Task {
      override def run(): Unit = {
        listener.asInstanceOf[TaskListener[Any]].taskFinished(arg, r)
      }
    })
  }

  override def dequeue(): Task = {
    if (q.isEmpty) {
      null
    } else {
      q.poll()
    }
  }
}

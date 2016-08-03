package com.thunderhead.mock.fabric

import java.util.concurrent.ConcurrentLinkedQueue

import com.thunderhead.core.TaskListener
import com.thunderhead.core.conf.LocalEnvironment
import com.thunderhead.core.fabric._

/**
  * Created by mike on 7/25/16.
  */
class SingleThreadMockFabric(count: Int) extends LocalEnvironment {
  class InternalNodeHandle(num: Int) extends NodeHandle

  class InternalGateway(index: Int) extends Gateway {
    val q = new ConcurrentLinkedQueue[MessagePacket]()

    override def send(obj: OutgoingMessage[_], id: Int, target: NodeHandle): Unit = {
      q.add(new MessagePacket {
        override def message(): OutgoingMessage[_] = obj
        override def exists(): Boolean = true
        override def sender(): NodeHandle = new InternalNodeHandle(index)
        override def taskId(): Int = id
      })
    }

    override def recv(): MessagePacket = {
      val r = q.peek()

      if (r == null) {
        new MessagePacket {
          override def message(): OutgoingMessage[_] = throw IllegalStateException
          override def exists(): Boolean = false
          override def sender(): NodeHandle = throw IllegalStateException
          override def taskId(): Int = throw IllegalStateException
        }
      } else {
        r
      }
    }
  }

  val map: Array[Gateway] = (for (i <- 0 until count) yield new InternalGateway(i)).toArray
}

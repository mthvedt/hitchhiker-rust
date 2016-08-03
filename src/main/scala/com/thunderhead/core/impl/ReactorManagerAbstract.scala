//package com.thunderhead.core.impl
//
//import com.thunderhead.core
//import com.thunderhead.core.{ReactorManager, ReactorTask}
//import com.thunderhead.core.fabric.{Codec, Gateway}
//
//import scala.collection.mutable
//
///**
//  * Created by mike on 7/25/16.
//  */
//class ReactorManagerAbstract(g: Gateway) extends ReactorManager {
//  val q: mutable.Queue[ReactorTask] = mutable.Queue.empty
//
//  class SavedTask[T](taskListener: TaskListener[T], codec: Codec[T]) {
//    def executeWithObjResponse(o: Any) = taskListener.continue(o.asInstanceOf[T], reactor())
//    def executeWithByteResponse(bytes: Any) = throw new IllegalStateException("not implemented")
//  }
//
//  // TODO: faster if circular buffer?
//  val waiterMap: mutable.Map[Int, SavedTask[_]] = mutable.HashMap.empty
//  var taskBegin: Int = 0
//  var taskEnd: Int = 0
//
//  val myReactor = new Reactor {
//    override def trampoline[T](t: T, responder: TaskListener[T]): Unit = {
//      q.enqueue(new ReactorTask{
//        override def execute(): Unit = {
//          responder.continue(t, reactor())
//        }
//      })
//    }
//
//    override def trampolineError[T](err: Object, responder: TaskListener[T]): Unit = {
//      q.enqueue(new ReactorTask{
//        override def execute(): Unit = {
//          responder.handleError(err, reactor())
//        }
//      })
//    }
//
//    override def netSend[T](request: Message, target: NodeHandle,
//                            responder: TaskListener[T], codec: Codec[T]) = {
//      if (taskBegin == taskEnd) {
//        q.enqueue(new ReactorTask {
//          override def execute(): Unit = reactor().netSend[T](request, target, responder, codec)
//        })
//      } else {
//        val taskNum = taskBegin
//        waiterMap.put(taskNum, new SavedTask[T](responder, codec))
//        g.send(request, taskNum, target)
//
//        taskBegin += 1
//      }
//    }
//
//    // TODO ManifoldInfoImpl -> ManifoldInfo
//    override def manifoldInfo(): ManifoldInfoImpl = ???
//  }
//
//  def drainIoQueues() = {
//    var done = false
//
//    while (!done) {
//      val p = g.recv()
//      if (p.exists()) {
//        val task = waiterMap(p.taskId())
//        // TODO this is wrong
//        // TODO do we need p to have a sender id?
//        task.executeWithObjResponse(p.message())
//      } else {
//        done = true
//      }
//    }
//  }
//
//  override def reactor() = myReactor
//
//  override def nextTask(): Option[ReactorTask] = {
//    drainIoQueues()
//
//    q.isEmpty match {
//      case true => None
//      case false => Some(q.dequeue())
//    }
//  }
//
//  override def doneWithTask() = {}
//}

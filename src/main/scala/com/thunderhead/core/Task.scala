package com.thunderhead.core

import com.thunderhead.core.reactor.{Reactor, TaskError, TaskListener}

import scala.reflect.ClassTag

/**
  * Created by mike on 7/22/16.
  */
abstract class Task[A] {
  // Brief refresher: A monad m has a box function a => m[a], called apply/(constructor) in scala,
  // and a bind function bind(m[a], f: a => m[b]) => m[b], called flatMap in scala.
  // We require that:
  // 1) flatMap(m, apply) == m
  // 2) flatMap(apply(x), f) == f(x)
  // 3) flatMap(flatMap(a, f1), f2) == flatMap(a, \x -> flatMap(f1(x), f2))

  def map[B](f: A => B): Task[B] = {
    val thisCapture: Task[A] = this

    new Task[B] {
      override def executeWith(r: Reactor, cont: TaskListener[B]): Unit = {
        thisCapture.executeWith(r, new TaskListener[A] {
          override def taskFinished(a: A, r: Reactor): Unit = cont.taskFinished(f(a), r)

          override def taskError(error: TaskError, r: Reactor): Unit = cont.taskError(error, r)
        })
      }
    }
  }

  def flatMap[B](f: A => Task[B]): Task[B] = {
    // TODO is there a better way to do this?
    val thisCapture: Task[A] = this

    new Task[B] {
      override def executeWith(r: Reactor, cont: TaskListener[B]): Unit = {
        thisCapture.executeWith(r, new TaskListener[A] {
          override def taskFinished(a: A, r: Reactor): Unit = f(a).executeWith(r, cont)

          override def taskError(error: TaskError, r: Reactor): Unit = cont.taskError(error, r)
        })
      }
    }
  }

  //  def withFilter(p: A => Boolean) TODO what to put here?
//  def foreach[U](f: A => U): Unit TODO what to put here?
  def executeWith(r: Reactor, responder: TaskListener[A])
}

object Task {
  // We use this to implement the box function.
  class TaskBox[A](a: A) extends Task[A] {
    // Easy to see the monad laws hold here.
    override def flatMap[B](f: (A) => Task[B]) = f(a)

    override def executeWith(r: Reactor, cont: TaskListener[A]) = r.forward(a, cont)
  }

  // TODO error types
  class ErrorTask[A](err: TaskError) extends Task[A] {
    //  def withFilter(p: A => Boolean) TODO what to put here?
    override def flatMap[B](f: (A) => Task[B]): Task[B] = this.asInstanceOf[Task[B]]
    override def executeWith(r: Reactor, cont: TaskListener[A]): Unit = r.forwardError(err, cont)
  }

  // TODO do something with this
  class ComboTask[T](rs: Task[T]*)(implicit m: ClassTag[T]) extends Task[Seq[T]] {
    override def executeWith(reactor: Reactor, cont: TaskListener[Seq[T]]) = {
      val ts = new Array[T](rs.length)
      var countdown = rs.length

      for ((r, i) <- rs.zipWithIndex) {
        r.executeWith(reactor, new TaskListener[T] {
          override def taskFinished(t: T, r: Reactor): Unit = {
            countdown -= 1
            ts(i) = t
            if (countdown == 0) {
              cont.taskFinished(ts, r)
            }
          }

          // TODO trampoline error instead?
          override def taskError(err: TaskError, r: Reactor): Unit = r.forwardError(err, cont)
        })
      }
    }
  }

  def apply[A](a: A): Task[A] = new TaskBox(a)
}

object test {
  val a = 5
  val b = 2

  val z = for {
    x <- Task(a)
    y <- Task(b)
  } yield x * y
}

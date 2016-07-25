package com.thunderhead.core

/**
  * Created by mike on 7/22/16.
  */
abstract class Response[A] {
  // Brief refresher: A monad m has a box function a => m[a], called apply/(constructor) in scala,
  // and a bind function bind(m[a], f: a => m[b]) => m[b], called flatMap in scala.
  // We require that:
  // 1) flatMap(m, apply) == m
  // 2) flatMap(apply(x), f) == f(x)
  // 3) flatMap(flatMap(a, f1), f2) == flatMap(a, \x -> flatMap(f1(x), f2))

  def flatMap[B](f: A => Response[B]): Response[B] = {
    // TODO is there a better way to do this?
    val thisCapture: Response[A] = this

    new Response[B] {
      override def executeWith(r: Reactor, cont: ResponseListener[B]): Unit =
        thisCapture.executeWith(r, new ResponseListener[A] {
          override def continue(a: A, r: Reactor): Unit = f(a).executeWith(r, cont)
          override def handleError(error: Object, r: Reactor): Unit = cont.handleError(error, r)
        })
    }
  }

  //  def withFilter(p: A => Boolean) TODO what to put here?
//  def foreach[U](f: A => U): Unit TODO what to put here?
  def executeWith(r: Reactor, responder: ResponseListener[A])
}

object Response {
  // We use this to implement the box function.
  class ResponseBox[A](a: A) extends Response[A] {
    // Easy to see the monad laws hold here.
    override def flatMap[B](f: (A) => Response[B]) = f(a)

    override def executeWith(r: Reactor, cont: ResponseListener[A]) = r.trampoline(a, cont)
  }

  // TODO error types
  class ResponseError[A](err: Object) extends Response[A] {
    //  def withFilter(p: A => Boolean) TODO what to put here?
    override def flatMap[B](f: (A) => Response[B]): Response[B] = this.asInstanceOf[Response[B]]
    override def executeWith(r: Reactor, cont: ResponseListener[A]): Unit = r.trampolineError(err, cont)
  }

  // TODO do something with this
  class ComboResponse[T](rs: Response[T]*) extends Response[Seq[T]] {
    override def executeWith(reactor: Reactor, cont: ResponseListener[Seq[T]]) = {
      val ts = new Array[T](rs.length)
      var countdown = rs.length

      for ((r, i) <- rs.zipWithIndex) {
        r.executeWith(reactor, new ResponseListener[T] {
          override def continue(t: T, r: Reactor): Unit = {
            countdown -= 1
            ts(i) = t
            if (countdown == 0) {
              cont.continue(ts, r)
            }
          }

          // TODO trampoline error instead
          override def handleError(err: Object, r: Reactor): Unit = r.trampolineError(err, cont)
        })
      }
    }
  }

  def apply[A](a: A): Response[A] = new ResponseBox(a)
}

object test {
  val a = 5
  val b = 2

  val z = for {
    x <- Response(a)
    y <- Response(b)
  } yield x * y
}

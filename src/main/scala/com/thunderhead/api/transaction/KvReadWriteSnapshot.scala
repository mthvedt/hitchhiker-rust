package com.thunderhead.api.transaction

import com.thunderhead.api.Datum
import com.thunderhead.core.Task

/**
  * Created by mike on 7/23/16.
  */
trait KvReadWriteSnapshot extends KvSnapshot {
  def write(k: Datum, v: Datum): Task[Range]

  def delete(k: Datum): Task[Range]

  def saveAndClose(): Task[Unit]
}

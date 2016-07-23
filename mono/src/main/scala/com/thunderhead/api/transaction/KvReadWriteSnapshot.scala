package com.thunderhead.api.transaction

import com.thunderhead.api.Datum
import com.thunderhead.core.Response

/**
  * Created by mike on 7/23/16.
  */
trait KvReadWriteSnapshot extends KvSnapshot {
  def write(k: Datum, v: Datum): Response[(Counter, Counter)]

  def saveAndClose(): Response[Unit]
}

package com.thunderhead.api.transaction

import com.thunderhead.api.Datum
import com.thunderhead.core.Response

/**
  * Created by mike on 7/23/16.
  */
trait KvReadWriteSnapshot extends KvSnapshot {
  def write(k: Datum, v: Datum): Response[Range]

  def delete(k: Datum): Response[Range]

  def saveAndClose(): Response[Unit]
}

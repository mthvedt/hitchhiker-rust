package com.thunderhead.api.transaction

import com.thunderhead.api.Datum
import com.thunderhead.core.Response

/**
  * Created by mike on 7/23/16.
  */
trait KvSnapshot {
  def read(k: Datum): Response[(Datum, Counter, Counter)]

  def dispose()
}

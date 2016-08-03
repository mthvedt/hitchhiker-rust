package com.thunderhead.api.transaction

import com.thunderhead.api.Datum
import com.thunderhead.core.Task$

/**
  * Created by mike on 7/23/16.
  */
trait KvSnapshot {
//  def read(k: Datum): Task[(Datum, Range)]

  def dispose()
}

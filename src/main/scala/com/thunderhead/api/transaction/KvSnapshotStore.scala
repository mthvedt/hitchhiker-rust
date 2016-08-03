package com.thunderhead.api.transaction

import com.thunderhead.core.Task

/**
  * Created by mike on 7/22/16.
  */
trait KvSnapshotStore {
  def open(): KvReadWriteSnapshot

  def delete(snapshotId: Counter): Task[Unit]
}

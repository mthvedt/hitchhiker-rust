package com.thunderhead.api.transaction

import com.thunderhead.core.Response

/**
  * Created by mike on 7/22/16.
  */
trait KvSnapshotStore {
  def open(): KvReadWriteSnapshot

  def delete(snapshotId: Counter): Response[Unit]
}

package com.thunderhead.core.conf

import com.thunderhead.core.{Reactor, ReactorManager}
import com.thunderhead.core.fabric.{DiskDevice, Gateway, NetDevice}

/**
  * Created by mike on 7/26/16.
  */
trait MachineConfig {
  /**
    * The number of reactors. In production, there is one per CPU executable thread.
    * TODO: make NUMA aware.
    * @return
    */
  def reactors(): IndexedSeq[Reactor]
  def interfaces(): IndexedSeq[NetDevice]
  def disks(): IndexedSeq[DiskDevice]
}

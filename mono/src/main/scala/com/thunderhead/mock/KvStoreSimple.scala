package com.thunderhead.mock

import com.thunderhead.api.Datum
import com.thunderhead.api.transaction.{Counter, KvReadWriteSnapshot, KvSnapshotStore, Range, TransactionView}
import com.thunderhead.core.Task$
import com.thunderhead.core.kv.local.{TransactionHandle, TransactionalStore, TxTracker}

import scala.collection.mutable.{Map => MMap}
import scala.collection.immutable.{Map => IMap}

/**
  * A dumb-as-possible implementation of a key value store.
  * Does not support snapshot ranges or space deletion (always keeps a tombstone for deleted values).
  */
class KvStoreSimple extends KvSnapshotStore {
  // datum is a value, or a 'tombstone' null value if this entry was created and deleted sometime in the past.
  // txVersion should be updated for every transaction affecting this key, even if no change
  // (principle: test classes should show most conservative behavior).
  class Entry(val datum: Option[Datum], val txVersion: Int) {
  }

  val snapshots: MMap[Int, IMap[Datum, Entry]] = MMap.empty
//  val transactions: MMap[TransactionHandle, TransactionView] = MMap.empty
  var startTx: Int = 0
  var endTx: Int = 0

  def internalOpenTransaction(): KvReadWriteSnapshot = new KvReadWriteSnapshot {
    val mySnapshot: IMap[Datum, Entry] = snapshots(endTx)
    val reads: MMap[Datum, Entry] = MMap.empty
    val writes: MMap[Datum, Option[Datum]] = MMap.empty

//    override def get(key: Datum): Datum = mySnapshot.get(key) match {
//      case Some(entry) => {
//        reads.put(key, entry)
//        entry.datum match {
//          case Some(value) => value
//          case None => null
//        }
//      }
//      case None => null
//    }
//
//    override def put(key: Datum, value: Datum) = {
//      writes.put(key, Some(value))
//    }
//
//    override def delete(key: Datum) = {
//      writes.put(key, None)
//    }
//
//    override def close(): Unit = {
//
//    }
    override def read(k: Datum): Task[(Datum, Range)] = ???

    override def write(k: Datum, v: Datum): Task[Range] = ???

    override def delete(k: Datum): Task[Range] = ???

    override def saveAndClose(): Task[Unit] = ???

    override def dispose(): Unit = {}
  }

  def openTransaction(): TransactionView = {
    val t = internalOpenTransaction()
//    val h = TransactionHandle
//    transactions.put(h, t)
    t
  }

//  override def getTransaction(stamp: TransactionHandle): TxTracker = ???
  override def open(): KvReadWriteSnapshot = ???

  override def delete(snapshotId: Counter): Task[Unit] = ???
}

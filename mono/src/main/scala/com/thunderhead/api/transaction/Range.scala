package com.thunderhead.api.transaction

/**
  * Created by mike on 7/23/16.
  */
trait Range {
  def min: Option[Counter]
  def max: Option[Counter]
  def isMaxed: Boolean
}

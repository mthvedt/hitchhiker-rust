use std;
use std::fmt;

use data::*;

// TODO: consider making this 128, or var-sized

/// A 64-bit counter. Counters wrap around and are read/written in big-endian format.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Counter {
    data: u64,
}

impl Counter {
    /// Creates a new Counter wrapping the given unsigned int.
    pub fn new(x: u64) -> Counter {
        Counter { data: x }
    }

    /// Increments this counter by one, wrapping around to 0 if needed.
    pub fn inc(self) -> Counter {
        Counter { data: self.data.wrapping_add(1) }
    }

    /// Returns this counter as an array of bytes, in big-endian.
    pub fn to_bytes(self) -> [u8; 8] {
        unsafe { std::mem::transmute(self.data.to_be()) }
    }

    /// Returns true if this counter is less than the given counter. A counter x is 'less than'
    /// a counter y if y - x < u64::max_value() / 2 - 1, using wrapping arithmetic.
    /// Informally, x must be "behind" y by less than maximum distance.
    // TODO unit test these.
    pub fn circle_lt(self, other: Counter) -> bool {
        return other.data.wrapping_sub(self.data).wrapping_sub(1) < u64::max_value() / 2 - 1;
    }

    /// Returns true if this counter is less than or equal to the given counter. A counter x is 'less than'
    /// a counter y if y - x < u64::max_value() / 2 - 1, using wrapping arithmetic.
    /// Informally, x must be "behind" y by less than maximum distance.
    pub fn circle_lt_eq(self, other: Counter) -> bool {
        return other.data.wrapping_sub(self.data) < u64::max_value() / 2;
    }
}

// TODO impl Key
impl Datum for Counter {
    fn len(&self) -> usize {
        8
    }

    fn write_bytes<W: DataWrite>(&self, w: W) -> W::Result {
        w.write(&self.to_bytes())
    }
}

impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.data)
    }
}

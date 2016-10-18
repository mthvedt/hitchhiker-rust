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
    pub fn new(x: u64) -> Counter {
        Counter { data: x }
    }

    pub fn inc(self) -> Counter {
        Counter { data: self.data.wrapping_add(1) }
    }

    pub fn bytes(self) -> [u8; 8] {
        unsafe { std::mem::transmute(self.data.to_be()) }
    }

    pub fn circle_lt(self, other: Counter) -> bool {
        return other.data.wrapping_sub(self.data).wrapping_sub(1) < u64::max_value() / 2 - 1;
    }

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
        w.write(&self.bytes())
    }
}

impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.data)
    }
}

//! Utility fns and macros.

use std::cmp::{min, max};
use std::io;
use std::io::{Read, Write};
use std::ptr::copy_nonoverlapping;

/// Make an array, populating each element according to the given constructor, which should be a lambda of one int.
#[macro_export]
macro_rules! make_array {
    ($constructor: expr, $n: expr) => {
        {
            let mut items: [_; $n] = mem::uninitialized();
            for (i, place) in items.iter_mut().enumerate() {
                ptr::write(place, $constructor(i));
            }
            items
        }
    }
}

// TODO: move this stuff to a 'byte' lib. Code guideline is util libs should be private
pub struct ByteReader<'a> {
    bytes: &'a [u8],
    ptr: usize,
}

impl<'a> ByteReader<'a> {
    /// Wraps the given slice in a ByteReader. This ByteReader reads the underlying bytes,
    /// starting at position 0.
    pub fn wrap(bytes: &'a [u8]) -> Self {
        ByteReader {
            bytes: bytes,
            ptr: 0,
        }
    }
}

// TODO unit test
impl<'a> Read for ByteReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = min(self.bytes.len() - self.ptr, buf.len());
        let len = max(len, 0);
        unsafe { copy_nonoverlapping(&self.bytes[self.ptr], &mut buf[0], len) };
        self.ptr += len;
        Ok(len)
    }
}

/// An implementation of Write that writes to a mutable byte buffer.
pub struct ByteWriter<'a> {
    v: &'a mut [u8],
    ptr: usize,
}

impl<'a> ByteWriter<'a> {
    /// Wraps the given slice in a ByteWriter. This ByteWriter writes to the underlying bytes,
    /// starting at position 0.
    pub fn wrap(underlying: &'a mut [u8]) -> ByteWriter<'a> {
        ByteWriter {
            v: underlying,
            ptr: 0,
        }
    }

    /// The number of bytes written to so far.
    pub fn len(&self) -> usize {
        self.ptr
    }
}

impl<'a> Write for ByteWriter<'a> {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if input.len() > self.v.len() - self.ptr {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "exceeded bounds of buffer in ByteWriter"));
        }

        unsafe {
            copy_nonoverlapping(&input[0], &mut self.v[self.ptr], input.len());
        }

        self.ptr += input.len();

        Ok(input.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

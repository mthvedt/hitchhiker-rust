use thunderhead_store::{KvSource, KvSink, Range, Source, Sink};
use thunderhead_store::alloc::Scoped;

use lens::{ReadLens, WriteLens};

pub struct MapStore<S, L> {
    underlying: S,
    lens: L,
}

impl<S, L> MapStore<S, L> {
    pub fn new(s: S, l: L) -> Self {
        MapStore {
            underlying: s,
            lens: l,
        }
    }
}

impl<S, L, T> Source<T> for MapStore<S, L> where
S: KvSource,
L: ReadLens<S>,
L::Target: Scoped<T>,
T: ?Sized + 'static
{
    type Get = L::Target;
    type GetF = L::ReadResult;

    fn get<K: Scoped<[u8]>>(&mut self, k: K) -> Self::GetF {
        let mut subtree = self.underlying.subtree(k);
        self.lens.read(subtree)
    }

    // type GetMany: Stream<Item = Self::GetValue, Error = io::Error>;
    // fn get_many<K: Scoped<[u8]>, I: IntoIterator<Item = K>>(&mut self, i: I) -> Self::GetMany {

    // }

    // type GetRange: Stream<Item = Self::GetValue, Error = io::Error>;
    // fn get_range<K: Scoped<[u8]>>(&mut self, range: Range) -> Self::GetRange {

    // }

    fn subtree<K: Scoped<[u8]>>(&mut self, k: K) -> Self {
        MapStore {
            underlying: self.underlying.subtree(k),
            lens: self.lens.clone(),
        }
    }

    fn subrange(&mut self, range: Range) -> Self {
        MapStore {
            underlying: self.underlying.subrange(range),
            lens: self.lens.clone(),
        }
    }
}

impl<S, L, T> Sink<<L as WriteLens<S>>::Target> for MapStore<S, L> where
S: KvSink,
L: ReadLens<S> + WriteLens<S, Target = T>,
<L as ReadLens<S>>::Target: Scoped<T>,
T: 'static,
{
    type PutF = L::WriteResult;

    fn max_value_size(&self) -> u64 {
        self.underlying.max_value_size()
    }

    fn put_small<K: Scoped<[u8]>, V: Scoped<T>>(&mut self, k: K, v: V) -> Self::PutF {
        let mut subtree = self.underlying.subtree(k);
        // TODO: This should only succeed if the subtree is empty.
        self.lens.write(v, subtree)
    }
}

#[cfg(test)]
mod test {

}

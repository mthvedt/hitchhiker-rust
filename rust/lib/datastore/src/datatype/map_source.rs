use thunderhead_store::{KvSource, KvSink, Range, Source, Sink};
use thunderhead_store::alloc::Scoped;
// TODO: move FutureResult, Scoped up a level?
use thunderhead_store::tdfuture::FutureResult;

use datatype::io::Lens;

// struct MapSource<S: KvSource, L: Lens<S>> {
//     underlying: S,
//     lens: L,
// }

// impl<S: KvSource, L: Lens<S>> Source<L::Target> for MapSource<S, L> {
//     type GetValue = L::Target;
//     type Get = L::ReadResult;

//     fn get<K: Scoped<[u8]>>(&mut self, k: K) -> FutureResult<Self::Get> {
//         let mut subtree = self.underlying.subtree(k);
//         self.lens.read(&mut subtree)
//     }

//     // type GetMany: Stream<Item = Self::GetValue, Error = io::Error>;
//     // fn get_many<K: Scoped<[u8]>, I: IntoIterator<Item = K>>(&mut self, i: I) -> Self::GetMany {

//     // }

//     // type GetRange: Stream<Item = Self::GetValue, Error = io::Error>;
//     // fn get_range<K: Scoped<[u8]>>(&mut self, range: Range) -> Self::GetRange {

//     // }


//     fn subtree<K: Scoped<[u8]>>(&mut self, k: K) -> Self {
//         MapSource {
//             underlying: self.underlying.subtree(k),
//             lens: self.lens.clone(),
//         }
//     }

//     fn subrange(&mut self, range: Range) -> Self {
//         MapSource {
//             underlying: self.underlying.subrange(range),
//             lens: self.lens.clone(),
//         }

//     }
// }

// impl<S: KvSink, L: Lens<S>> Sink<L::Target> for MapSource<S, L> {
//     type PutSmall = L::WriteResult;

//     fn max_value_size(&self) -> u64 {
//         self.underlying.max_value_size()
//     }

//     fn put_small<K: Scoped<[u8]>, V: Scoped<L::Target>>(&mut self, k: K, v: V) -> FutureResult<Self::PutSmall> {
//         let mut subtree = self.underlying.subtree(k);
//         // TODO: assert is empty, or erase
//         self.lens.write(v, &mut subtree)
//     }
// }

#[cfg(test)]
mod tests {

}

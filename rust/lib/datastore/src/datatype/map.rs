// trait DatatypeVerifier {
//     fn verify(d: DatatypeId) -> bool;
// }
//
// struct SingleDatatypeStore<S, D> {
//     inner: S,
//     verifier: D,
// }
//
// impl<S, V> RestSource for SingleRestStore<S, V> {
//     pub fn new(s: S) -> Self {
//         // TODO check
//         new_unchecked(s)
//     }
//
//     fn new_unchecked(s: S) -> Self {
//         SingleRestStore {
//             inner: s,
//             _p: PhantomData,
//         }
//     }
// }
//
// // TODO use lenses
// impl<S, V> RestSource for SingleRestStore<S, V> where
// S: KvSource,
// V: RestDatatypeId,
// {
//     type Get = RestResourceGet<S::Get>;
//
//     fn get<K: Scoped<[u8]>>(&mut self, k: K) -> Self::GetF {
//         RestResourceGet {
//             inner: self.inner.get(k),
//         }
//     }
//
//     fn subtree<K: Scoped<[u8]>>(&mut self, k: K) -> Self {
//         Self::new_unchecked(self.inner.subtree(k))
//     }
//
//     fn subrange(&mut self, range: Range) -> Self {
//         Self::new_unchecked(self.inner.subrange(range))
//     }
// }

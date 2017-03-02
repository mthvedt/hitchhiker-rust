//! tdfuture is the futures apparatus for Thunderhead. It provides a few mechanisms
//! for better managing futures in Thunderhead.

// /// A TdTask is a continuation: it accepts an input value or an error value,
// /// and returns no result. Unlike first-class continuations, a TdTask can only
// /// be run once by default.
// trait TdLink<Input> {
//     fn execute
// }

/// A pseudo higher-kinded type in Rust.
///
/// A PHKT is any generic type where the generic argument can be 'swapped' for a different one.
/// A PHKT<U> has a member, Target, that represents Self<U>.
/// The usefulness of PHKTs is in writing extension traits. For instance, we can write the following:
///
/// ```
/// trait Functor<U>: HKT<U> {
///     fn map<F>(&self, f: F) -> Self::T where F: Fn(&Self::C) -> U;
/// }
///
/// impl<T, U> Functor<U> for Vec<T> {
///     fn map<F>(&self, f: F) -> Vec<U> where F: Fn(&T) -> U {
///         let mut result = Vec::with_capacity(self.len());
///         for value in self {
///             result.push( f(value) );
///         }
///         result
///     }
/// }
/// ```
///
/// We now have a Functor<U> for any Vec<T>.

trait PHKT<U> {
    type C;
    type T;
}

macro_rules! derive_hkt {
    ($t:ident) => {
        impl<T, U> PHKT<U> for $t<T> {
            type C = T;
            type T = $t<U>;
        }
    }
}

///! Pseudo-HKTs in Rust. See `PHKT`.

/// A pseudo higher-kinded type in Rust.
///
/// A PHKT is any generic type where the generic argument can be 'swapped' for a different one.
/// A PHKT<U> has a member, Target, that represents Self<U>.
/// The usefulness of PHKTs is in writing extension traits. For instance, we can write the following:
///
/// ```
/// # #[macro_use]
/// # extern crate htree;
///
/// # use htree::tdfuture::phkt;
/// # use htree::tdfuture::phkt::*;
///
/// # fn main() {
///
/// trait Functor<U>: PHKT<U> {
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
///
/// # }
/// ```
///
/// We now have a Functor<U> for any Vec<T>.
pub trait PHKT<U> {
    type C;
    type T;
}

/// Shortcut to impl a PHKT<U> for a generic type.
#[macro_export]
macro_rules! derive_phkt {
    ($t:ident) => {
        impl<T, U> PHKT<U> for $t<T> {
            type C = T;
            type T = $t<U>;
        }
    }
}

derive_phkt!(Vec);

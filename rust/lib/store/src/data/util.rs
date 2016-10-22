//! Utility fns and macros.

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

# A note on shared pointers

Any exposure of a shared pointer must be done via *mutable* borrow,
even if it will be passed to `jsapi` via a `*const`. This is because
Rust has less restrictive antialiasing than C++, so to prevent our brain
hurting from thinking about cross-language antialiasing, we just only allow
pointers to never be aliased. Simple.

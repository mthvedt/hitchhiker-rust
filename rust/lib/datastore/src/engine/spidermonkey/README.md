# A note on shared pointers

Any exposure of a shared pointer must be done via *mutable* borrow,
even if it will be passed to `jsapi` via a `*const`. This is because
Rust has less restrictive antialiasing than C++, so to prevent our brain
hurting from thinking about cross-language antialiasing, we just only allow
pointers to never be aliased. Simple. (In particular, Rust does not have a well-defined or even well-documented
memory model, so we cannot be sure how it will treat aliased pointers across C++ calls).

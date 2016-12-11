use std::collections::HashMap;

pub enum NativeValue {
    Bool(bool),
    Double(f64),
    Int(i32),
    Null,
    Object(HashMap<NativeValue, NativeValue>),
    String(String),
}

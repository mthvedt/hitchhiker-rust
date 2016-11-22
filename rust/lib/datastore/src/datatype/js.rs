// PLAN FOR JS:
//
// KV--too heavyweight and difficult. Also, we usually read whole JSON documents.
// So... flat files it is.

// Design: A 'wire' type, a 'binary' type, and a 'JSON' type, and lenses for each.
// Some kinda combined lens can yield both.

/// A Json-wrapping type. It's intentionally opaque, since its inner fields are not intended for Rust.
/// (Not yet anyway.)
struct TdJson {

}

// This is not yet supported. Instead we use SpiderMonkey directly.
// /// A lens for text JSON, which is what TD currently sends/receives over the wire.
// struct RestJsonLens;

// impl<S: KvSink> Lens<S> for TextJsonLens {
//     type Target = String;

//     type ReadResult: Future<Item = Self::Target, Error = io::Error>;

//     fn read(&self, source: &mut S) -> FutureResult<Self::ReadResult> {

//     }

//     type WriteResult: Future<Item = (), Error = io::Error>;

//     fn write<V: Scoped<Self::Target>>(&self, target: V, sink: &mut S) -> FutureResult<Self::WriteResult> {

//     }
// }

/// A lens that turns binary JSON blobs into Spidermonkey JS.
struct SmJsonLens;

/// A lens that turns REST wire-format JSON into Spidermonkey JS.
struct SmTextJsonLens;

#[cfg(test)]
mod test {

}

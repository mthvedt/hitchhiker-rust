use std::io::Write;

/// The most general way to serialize/deserialize a given data type to a KV subtree.
trait KvIo {
    type Target;

    fn read<S: KvSource>(&self, source: &S) -> Self::Target;

    fn write<S: KvSink>(&self, target: &Self::Target, sink: &S);
}

// TODO should serializers be objects?

// TODO: struct?
trait DatatypeHeader {
    fn id(&self) -> DatatypeId;

    fn version(&self) -> u32;

    /// Additional data that varies type by type.
    fn extended_data(&self) -> &[u8];
}

trait DatatypeRw {
    type Target;
    type Header: DatatypeHeader;
    type KvIo: KvIo<Target = Self::Target>;

    fn header(&self) -> Self::DatatypeHeader;

    fn sub_rw(&self) -> Self::KvIo;
}

struct SmallRwReifiedIo<T: SmallRw> {
    t: T,
}

impl<T: SmallRw> KvIo for SmallRwReifiedIo<T> {
    type Target = T::Target;

    fn read<S: KvSource>(&self, s: &S) -> Self::Target {
        let slice: [u8; 1] = [1];
        self.t.from_bytes(source.get(slice))
    }

    fn write<S: KvSink>(&self, target: &Self::Target, sink: &S) {
        let slice: [u8; 1] = [1];
        sink.put(slice, self.t.to_bytes(target))
    }
}

struct SmallRwReified<T: SmallRw> {
    t: T,
}

impl<T: SmallRw> DatatypeRw for SmallRwReified<T>  {
    type Target = T::Target;
    type Header = T::Header;
    type KvIo = SmallRwReified<T>;

    fn header(&self) -> Self::DatatypeHeader {
        self.t.header()
    }

    fn sub_rw(&self) -> Self::KvIo {
        SmallRwReifiedIo {
            t: self.t.clone(),
        }
    }
}

trait SmallRw: Clone {
    type Header: DatatypeHeader;
    type Target;

    fn header(&self) -> Self::Header;

    fn to_bytes(&self, t: &Self::Target) -> Box<[u8]>;

    fn from_bytes(&self, b: &[u8]) -> Self::Target;

    fn reify(&self) -> SmallRwReified<Self> {
        SmallRwReified {
            t: self.clone(),
        }
    }
}

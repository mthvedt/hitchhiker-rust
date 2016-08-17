rocksdb-thunderhead
===================

The rocksdb-thunderhead interface.

The reason this is its own crate is to separate the rocksdb backend
from the Thunderhead interface. There are a few motivations here:
1. the current rocksdb backend is third-party, and you should
always isolate third-party code;
2. the rocksdb FFI should be in a separate crate anyway.

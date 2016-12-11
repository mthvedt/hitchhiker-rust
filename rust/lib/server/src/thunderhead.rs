

/// A Thunderhead REST store.
pub trait TdRequestHandler {
    type ResultF: Future<Item = TdResponse, Error = TdError>;

    fn handle(&mut self, req: TdRequest) -> Self::ResultF;
}

struct TdJsRequestHandler {
    // TODO
}

struct TdRestStore {
    // TODO
}

// TODO: how to bootstrap?
// Master request handler reads a JsRequestHandler.

// impl TdRestStore {
//     fn handle(&mut self, req: TdRequest) -> TdResponse;
// }

// TODO: wire up a javascript handler and do it!

/*
Classes needed:

TdRequestHandler--just do the whole f'ing thing in JS

TdRestStore--for native storing of JSON s.t. we can add transactions
*/

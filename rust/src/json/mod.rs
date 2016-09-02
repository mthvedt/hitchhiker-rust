use std::io::Read;

use rustc_serialize::json::*;

// TODO: high-speed native rust handling of json goes here

fn decode_json(rdr: &mut Read) -> DecodeResult<Json> {
    // TODO: make this panic on any syntax error?
    Json::from_reader(rdr)
}

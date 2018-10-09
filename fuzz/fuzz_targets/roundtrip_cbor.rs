#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate ssb_legacy_msg;

use ssb_legacy_msg::cbor::{from_slice, to_vec};
use ssb_legacy_msg::json::Value;

fuzz_target!(|data: &[u8]| {
    // This comment keeps rustfmt from breaking the fuzz macro...
    match from_slice::<Value>(data) {
        Ok(val) => {
            let encoded = to_vec(&val);
            let redecoded = from_slice::<Value>(&encoded[..]).unwrap();
            assert_eq!(val, redecoded);
        }
        Err(_) => {}
    }
});

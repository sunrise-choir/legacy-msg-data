#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate ssb_legacy_msg_data;

use ssb_legacy_msg_data::{cbor, json};
use ssb_legacy_msg_data::value::Value;

fuzz_target!(|data: &[u8]| {
    // This comment keeps rustfmt from breaking the fuzz macro...
    match json::from_slice::<Value>(data) {
        Ok(val) => {
            let encoded = cbor::to_vec(&val).unwrap();
            let redecoded = cbor::from_slice::<Value>(&encoded[..]).unwrap();
            assert_eq!(val, redecoded);
        }
        Err(_) => {}
    }

    match cbor::from_slice::<Value>(data) {
        Ok(val) => {
            let encoded = json::to_vec(&val, true).unwrap();
            let redecoded = json::from_slice::<Value>(&encoded[..]).unwrap();
            assert_eq!(val, redecoded);
        }
        Err(_) => {}
    }
});

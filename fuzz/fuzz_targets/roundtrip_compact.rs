#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate ssb_json_msg_data;

use ssb_json_msg_data::{
    json::{from_slice, to_vec},
    value::Value,
};

fuzz_target!(|data: &[u8]| {
    // This comment keeps rustfmt from breaking the fuzz macro...
    match from_slice::<Value>(data) {
        Ok(val) => {
            let sign_json = to_vec(&val, true).unwrap();
            let redecoded = from_slice::<Value>(&sign_json[..]).unwrap();z
            assert_eq!(val, redecoded);
        }
        Err(_) => {}
    }
});

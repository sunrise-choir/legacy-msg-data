#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate ssb_legacy_msg_data;

use ssb_legacy_msg_data::cbor::{from_slice, to_vec};
use ssb_legacy_msg_data::value::Value;

fuzz_target!(|data: &[u8]| {
    // This comment keeps rustfmt from breaking the fuzz macro...
    match from_slice::<Value>(data) {
        Ok(val) => {
            match to_vec(&val) {
                Err(e) => {
                    println!("to_vec err: {:?}", e);
                    println!("{:x?}", &val);
                    panic!()
                }

                Ok(encoded) => {
                    match from_slice::<Value>(&encoded[..]) {
                        Err(e) => {
                            println!("from_slice err: {:?}", e);
                            println!("{:x?}", &val);
                            println!("{:x?}", &encoded);
                            panic!()
                        }

                        Ok(redecoded) => {
                            if val != redecoded {
                                println!("{:?}\n", val);
                                println!("{:?}\n", redecoded);
                                panic!();
                            }
                        }
                    }
                }
            }
        }
        Err(_) => {}
    }
});

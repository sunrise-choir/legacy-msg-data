//! TODO: Document and clean up.
//! TODO: link to spec

mod de;
mod ser;

pub use self::de::{CborDeserializer, from_slice, from_slice_partial};
pub use self::ser::{CborSerializer, to_writer, to_vec};

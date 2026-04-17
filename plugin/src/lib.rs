mod generated;
mod shared;

use wasm_minimal_protocol::{initiate_protocol, wasm_func};

initiate_protocol!();

/// A trait for types whose values can be encoded into bytes.
///
/// The encoded form should be able to be decoded to the original value, even
/// when followed by arbitrary bytes. For example, this means that vectors are
/// encoded with their length first.
trait Encode {
    /// The type of the encoded bytes.
    type Encoded: AsRef<[u8]>;

    /// Encodes a value into bytes.
    fn encode(&self) -> Self::Encoded;
}

impl<T: Encode + ?Sized> Encode for &T {
    type Encoded = T::Encoded;

    fn encode(&self) -> Self::Encoded {
        (*self).encode()
    }
}

impl Encode for u32 {
    type Encoded = [u8; 4];

    fn encode(&self) -> Self::Encoded {
        self.to_le_bytes()
    }
}

impl Encode for str {
    type Encoded = Vec<u8>;

    fn encode(&self) -> Self::Encoded {
        let mut bytes = (self.len() as u32).encode().to_vec();
        bytes.extend_from_slice(self.as_bytes());
        bytes
    }
}

impl<T: Encode> Encode for [T] {
    type Encoded = Vec<u8>;

    fn encode(&self) -> Vec<u8> {
        let mut bytes = (self.len() as u32).encode().to_vec();
        for item in self {
            bytes.extend_from_slice(item.encode().as_ref());
        }
        bytes
    }
}

/// A way to encode a tuple of [encodable](Encode) things, optionally terminated
/// with arbitrary data.
#[derive(Debug, Default)]
struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    /// Creates a new, empty encoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Encodes a new value after the already encoded data.
    pub fn push<T: Encode + ?Sized>(&mut self, x: &T) -> &mut Self {
        self.bytes.extend_from_slice(x.encode().as_ref());
        self
    }

    /// Returns the encoded tuple.
    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns the encoded tuple after pushing arbitrary data at the end.
    pub fn finish_with(mut self, data: impl AsRef<[u8]>) -> Vec<u8> {
        self.bytes.extend_from_slice(data.as_ref());
        self.bytes
    }
}

fn decode_codepoint(codepoint: &[u8]) -> Result<u32, &str> {
    Ok(u32::from_le_bytes(
        *codepoint.as_array().ok_or("invalid codepoint")?,
    ))
}

#[wasm_func]
pub fn get_block_data(codepoint: &[u8]) -> Result<Vec<u8>, &str> {
    let value = decode_codepoint(codepoint)?;
    match generated::block_data(value) {
        None => Ok(Vec::new()),
        Some(data) => {
            let mut encoder = Encoder::new();
            encoder.push(&data.first);
            encoder.push(&data.last);
            Ok(encoder.finish_with(data.name))
        }
    }
}

#[wasm_func]
pub fn get_codepoint_data(codepoint: &[u8]) -> Result<Vec<u8>, &str> {
    let value = decode_codepoint(codepoint)?;
    match generated::codepoint_data(value) {
        None => Ok(Vec::new()),
        Some(data) => {
            let mut encoder = Encoder::new();
            encoder.push(data.name);
            encoder.push(data.general_category);
            encoder.push(data.canonical_combining_class);
            encoder.push(generated::math_data(value).unwrap_or(""));
            Ok(encoder.finish())
        }
    }
}

#[wasm_func]
pub fn get_alias_data(codepoint: &[u8]) -> Result<Vec<u8>, &str> {
    let value = decode_codepoint(codepoint)?;
    let (corrections, controls, alternates, figments, abbreviations) = generated::alias_data(value);
    let mut encoder = Encoder::new();
    encoder.push(corrections);
    encoder.push(controls);
    encoder.push(alternates);
    encoder.push(figments);
    encoder.push(abbreviations);
    Ok(encoder.finish())
}

use crate::{Error, Result};

/// A type that can cross the Rack request/response body boundary.
///
/// `#[rack::payload]` implements this trait for JSON structs. The SDK also
/// provides implementations for `()` and `String`.
pub trait Payload: Sized {
    /// Build this payload from the raw HTTP request body bytes.
    fn from_body(body: &[u8]) -> Result<Self>;

    /// Serialize this payload into raw response body bytes.
    fn into_body(self) -> Result<Vec<u8>>;
}

impl Payload for () {
    fn from_body(_: &[u8]) -> Result<Self> {
        Ok(())
    }

    fn into_body(self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }
}

impl Payload for String {
    fn from_body(body: &[u8]) -> Result<Self> {
        String::from_utf8(body.to_vec()).map_err(|error| Error::new(error.to_string()))
    }

    fn into_body(self) -> Result<Vec<u8>> {
        Ok(self.into_bytes())
    }
}

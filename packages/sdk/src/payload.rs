use serde::{de::DeserializeOwned, Serialize};

use crate::{Error, Result};

pub trait Payload: Sized {
    fn from_body(body: &[u8]) -> Result<Self>;
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
        String::from_utf8(body.to_vec()).map_err(|error| Error::message(error.to_string()))
    }

    fn into_body(self) -> Result<Vec<u8>> {
        Ok(self.into_bytes())
    }
}

pub fn from_json<T: DeserializeOwned>(body: &[u8]) -> Result<T> {
    serde_json::from_slice(body).map_err(Error::from)
}

pub fn to_json<T: Serialize>(value: T) -> Result<Vec<u8>> {
    serde_json::to_vec(&value).map_err(Error::from)
}

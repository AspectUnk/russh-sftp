use std::mem::size_of;

use bytes::{Buf, BufMut};

use crate::{error::Error, protocol::StatusCode};

pub trait TryBuf: Buf {
    fn try_get_u8(&mut self) -> Result<u8, Error>;
    fn try_get_u32(&mut self) -> Result<u32, Error>;
    fn try_get_bytes(&mut self) -> Result<Vec<u8>, Error>;
    fn try_get_string(&mut self) -> Result<String, Error>;
}

impl<T: Buf> TryBuf for T {
    fn try_get_u8(&mut self) -> Result<u8, Error> {
        if self.remaining() < size_of::<u8>() {
            return Err(Error::Protocol(StatusCode::BadMessage));
        }

        return Ok(self.get_u8());
    }

    fn try_get_u32(&mut self) -> Result<u32, Error> {
        if self.remaining() < size_of::<u32>() {
            return Err(Error::Protocol(StatusCode::BadMessage));
        }

        return Ok(self.get_u32());
    }

    fn try_get_bytes(&mut self) -> Result<Vec<u8>, Error> {
        let len = self.try_get_u32()? as usize;
        if self.remaining() < len {
            return Err(Error::Protocol(StatusCode::BadMessage));
        }

        Ok(self.copy_to_bytes(len).to_vec())
    }

    fn try_get_string(&mut self) -> Result<String, Error> {
        let bytes = self.try_get_bytes()?;
        Ok(String::from_utf8(bytes).map_err(|_| Error::Protocol(StatusCode::BadMessage))?)
    }
}

pub trait PutBuf: BufMut {
    fn put_str(&mut self, str: &str);
}

impl<T: BufMut> PutBuf for T {
    fn put_str(&mut self, str: &str) {
        let bytes = str.as_bytes();

        self.put_u32(bytes.len() as u32);
        self.put_slice(bytes);
    }
}

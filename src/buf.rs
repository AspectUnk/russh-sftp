use bytes::{Buf, BufMut};
use std::mem::size_of;

use crate::error::Error;

pub trait TryBuf: Buf {
    fn try_get_u8(&mut self) -> Result<u8, Error>;
    fn try_get_u32(&mut self) -> Result<u32, Error>;
    fn try_get_u64(&mut self) -> Result<u64, Error>;
    fn try_get_bytes(&mut self) -> Result<Vec<u8>, Error>;
    fn try_get_string(&mut self) -> Result<String, Error>;
}

impl<T: Buf> TryBuf for T {
    fn try_get_u8(&mut self) -> Result<u8, Error> {
        if self.remaining() < size_of::<u8>() {
            return Err(Error::BadMessage("no remaining for u8".to_owned()));
        }

        Ok(self.get_u8())
    }

    fn try_get_u32(&mut self) -> Result<u32, Error> {
        if self.remaining() < size_of::<u32>() {
            return Err(Error::BadMessage("no remaining for u32".to_owned()));
        }

        Ok(self.get_u32())
    }

    fn try_get_u64(&mut self) -> Result<u64, Error> {
        if self.remaining() < size_of::<u64>() {
            return Err(Error::BadMessage("no remaining for u64".to_owned()));
        }

        Ok(self.get_u64())
    }

    fn try_get_bytes(&mut self) -> Result<Vec<u8>, Error> {
        let len = self.try_get_u32()? as usize;
        if self.remaining() < len {
            return Err(Error::BadMessage("no remaining for vec".to_owned()));
        }

        Ok(self.copy_to_bytes(len).to_vec())
    }

    fn try_get_string(&mut self) -> Result<String, Error> {
        let bytes = self.try_get_bytes()?;
        //String::from_utf8(bytes).map_err(|_| Error::BadMessage("unable to parse str".to_owned()))
        Ok(String::from_utf8_lossy(&bytes).into())
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

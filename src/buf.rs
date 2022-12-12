use std::mem::size_of;

use bytes::Buf;

use crate::error::Error;

pub trait TryBuf: Buf {
    fn try_get_u8(&mut self) -> Result<u8, Error>;
    fn try_get_u32(&mut self) -> Result<u32, Error>;
}

impl<T: Buf> TryBuf for T {
    fn try_get_u8(&mut self) -> Result<u8, Error> {
        if self.remaining() < size_of::<u8>() {
            return Err(Error::UnexpectedEof);
        }

        return Ok(self.get_u8());
    }

    fn try_get_u32(&mut self) -> Result<u32, Error> {
        if self.remaining() < size_of::<u32>() {
            return Err(Error::UnexpectedEof);
        }

        return Ok(self.get_u32());
    }
}

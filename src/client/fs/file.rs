use std::sync::Arc;
use tokio::sync::Mutex;

use super::Metadata;
use crate::client::{rawsession::SftpResult, RawSftpSession};

pub struct File {
    pub(crate) raw: Arc<Mutex<RawSftpSession>>,
    pub(crate) handle: String,
}

impl File {
    pub async fn metadata(&self) -> SftpResult<Metadata> {
        Ok(self
            .raw
            .lock()
            .await
            .fstat(self.handle.as_str())
            .await?
            .attrs)
    }

    pub async fn set_metadata(&self, metadata: Metadata) -> SftpResult<()> {
        self.raw
            .lock()
            .await
            .fsetstat(self.handle.as_str(), metadata)
            .await
            .map(|_| ())
    }
}

use std::sync::Arc;
use tokio::{runtime::Handle, sync::Mutex};

use super::Metadata;
use crate::client::{rawsession::SftpResult, RawSftpSession};

pub struct File {
    pub(crate) session: Arc<Mutex<RawSftpSession>>,
    pub(crate) handle: String,
}

impl File {
    pub async fn metadata(&self) -> SftpResult<Metadata> {
        Ok(self
            .session
            .lock()
            .await
            .fstat(self.handle.as_str())
            .await?
            .attrs)
    }

    pub async fn set_metadata(&self, metadata: Metadata) -> SftpResult<()> {
        self.session
            .lock()
            .await
            .fsetstat(self.handle.as_str(), metadata)
            .await
            .map(|_| ())
    }
}

impl Drop for File {
    fn drop(&mut self) {
        if let Ok(handle) = Handle::try_current() {
            let session = self.session.to_owned();
            let file_handle = self.handle.to_owned();

            handle.spawn(async move {
                let _ = session.lock().await.close(file_handle).await;
            });
        }
    }
}

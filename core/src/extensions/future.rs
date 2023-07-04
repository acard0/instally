use futures::Future;

pub trait FutureSyncExt {
    type Output;
    fn wait(self) -> Self::Output; 
}

impl<F, T> FutureSyncExt for F where F: Future<Output = T> {
    type Output = T;
    fn wait(self) -> Self::Output {
        
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            return handle.block_on(async {
                self.await
            });
        }
        
        tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            self.await
        })
   }
}
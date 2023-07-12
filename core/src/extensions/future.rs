use futures::Future;

pub trait FutureSyncExt {
    type Output;
    fn wait(self) -> Self::Output; 
}

impl<F, T> FutureSyncExt for F where F: Future<Output = T> {
    type Output = T;
    fn wait(self) -> Self::Output {
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current()
            .block_on(async move {
                self.await
            })
        })
   }
}
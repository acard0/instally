use futures::Future;

pub trait FutureSyncExt {
    type Output;
    fn wait(self) -> Self::Output; 
}

impl<F, T> FutureSyncExt for F where F: Future<Output = T> {
    type Output = T;
    fn wait(self) -> Self::Output {
        match tokio::runtime::Handle::try_current() {
            Ok(_) => {
                tokio::task::block_in_place(move || {
                    tokio::runtime::Handle::current()
                    .block_on(async move {
                        self.await
                    })
                })
            },

            _ => {
                tokio::runtime::Runtime::new().unwrap()
                    .block_on(self)
            }
        }
   }
}
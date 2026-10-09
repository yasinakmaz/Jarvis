//! Tek yazar aktör: bağlantının sahibi olan iş parçacığı (ADR 0024).

use std::thread;

use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot};

use crate::StoreError;

/// Aktörde çalışan iş.
type Job = Box<dyn FnOnce(&mut Connection) + Send>;

/// Bekleyen iş kuyruğunun üst sınırı (geri basınç).
const QUEUE: usize = 256;

/// Aktörün tutamacı.
#[derive(Clone, Debug)]
pub struct Actor {
    jobs: mpsc::Sender<Job>,
}

impl Actor {
    /// Bağlantıyı yeni bir iş parçacığına taşır. Son tutamaç düşünce iş parçacığı biter.
    pub fn spawn(conn: Connection) -> Result<Self, StoreError> {
        let (jobs, mut queue) = mpsc::channel::<Job>(QUEUE);
        thread::Builder::new()
            .name("jarvis-session".to_owned())
            .spawn(move || {
                let mut conn = conn;
                while let Some(job) = queue.blocking_recv() {
                    job(&mut conn);
                }
            })?;
        Ok(Self { jobs })
    }

    /// İşi aktörde çalıştırır ve sonucunu bekler. Çağıran vazgeçse de iş tamamlanır.
    pub async fn call<T, F>(&self, work: F) -> Result<T, StoreError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T, StoreError> + Send + 'static,
    {
        let (reply, result) = oneshot::channel();
        let job: Job = Box::new(move |conn| {
            // Alıcı düştüyse (çağıran vazgeçti) sonucu bekleyen yoktur.
            let _ = reply.send(work(conn));
        });
        self.jobs.send(job).await.map_err(|_| StoreError::Closed)?;
        result.await.map_err(|_| StoreError::Closed)?
    }
}

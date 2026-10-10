//! `OpenAI` ucu taklidi: kasetten tekrar eden yerel sunucu (Tasarım 0011).
//!
//! Üretim kodunda kaset dalı yoktur: sağlayıcının `base_url`'i bu sunucuya yöneltilir.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::Response;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::cassette::{CassetteError, Turn, first_difference, parse};
use crate::cassette_response::{failure, replay_response};

/// Sunucunun aldığı bir istek.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    /// HTTP yöntemi.
    pub method: String,
    /// Yol.
    pub path: String,
    /// Başlıklar (adlar küçük harf).
    pub headers: Vec<(String, String)>,
    /// Gövde; JSON değilse ham metin `Value::String` olarak.
    pub body: Value,
}

/// Tekrar sunucusu. Düşünce dinleme portu kapanır (`stop` göndericisi düşer ve zarif kapanışı
/// tetikler); süren bağlantılar kendi yanıtlarını bitirebilir.
pub struct CassetteServer {
    addr: SocketAddr,
    shared: Arc<Shared>,
    _stop: oneshot::Sender<()>,
    _task: JoinHandle<()>,
}

pub(crate) struct Shared {
    inner: Mutex<Inner>,
}

struct Inner {
    turns: Vec<Turn>,
    next: usize,
    recorded: Vec<Recorded>,
    problems: Vec<String>,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl CassetteServer {
    /// Kaset dosyasından tekrar eder.
    ///
    /// # Errors
    ///
    /// Dosya okunamaz ya da kaset geçersizse; ya da `127.0.0.1:0` dinlenemezse.
    pub async fn replay(path: &Path) -> Result<Self, CassetteError> {
        let text = std::fs::read_to_string(path).map_err(|e| CassetteError::Io {
            path: path.display().to_string(),
            detail: e.to_string(),
        })?;
        Self::replay_str(&text).await
    }

    /// Bellekteki kaset metninden tekrar eder (testlerin kendi sınaması için).
    ///
    /// # Errors
    ///
    /// [`CassetteServer::replay`] ile aynı.
    pub async fn replay_str(text: &str) -> Result<Self, CassetteError> {
        let turns = parse(text)?;
        if let Some(bad) = turns
            .iter()
            .find(|turn| StatusCode::from_u16(turn.response.status).is_err())
        {
            return Err(CassetteError::Replay(format!(
                "tur {}: geçersiz HTTP durumu {}",
                bad.turn, bad.response.status
            )));
        }
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| CassetteError::Bind(e.to_string()))?;
        let addr = listener
            .local_addr()
            .map_err(|e| CassetteError::Bind(e.to_string()))?;
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner {
                turns,
                next: 0,
                recorded: Vec::new(),
                problems: Vec::new(),
            }),
        });
        let app = Router::new()
            .fallback(handle)
            .with_state(Arc::clone(&shared));
        let (stop, stopped) = oneshot::channel::<()>();
        let background = Arc::clone(&shared);
        let task = tokio::spawn(async move {
            let serving = axum::serve(listener, app).with_graceful_shutdown(async move {
                // Gönderen düşse de kapanış sinyalidir.
                let _ = stopped.await;
            });
            if let Err(error) = serving.await {
                background
                    .lock()
                    .problems
                    .push(format!("sunucu durdu: {error}"));
            }
        });
        Ok(Self {
            addr,
            shared,
            _stop: stop,
            _task: task,
        })
    }

    /// Sağlayıcının `base_url`'i: `http://127.0.0.1:<port>/v1`.
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://{}/v1", self.addr)
    }

    /// Gelen gövdeler, geliş sırasıyla (oracle).
    #[must_use]
    pub fn requests(&self) -> Vec<Value> {
        self.recorded().into_iter().map(|r| r.body).collect()
    }

    /// Gelen isteklerin tamamı (başlıklar dahil).
    #[must_use]
    pub fn recorded(&self) -> Vec<Recorded> {
        self.shared.lock().recorded.clone()
    }

    /// Hiç uyuşmazlık olmadı ve her tur kullanıldı.
    ///
    /// # Errors
    ///
    /// Uyuşmazlık, tükenme ya da kullanılmayan tur varsa ayrıntılarıyla.
    pub fn assert_exhausted(&self) -> Result<(), CassetteError> {
        let inner = self.shared.lock();
        let unused = inner.turns.len().saturating_sub(inner.next);
        let mut problems = inner.problems.clone();
        if unused > 0 {
            problems.push(format!("kullanılmayan {unused} tur kaldı"));
        }
        drop(inner);
        if problems.is_empty() {
            Ok(())
        } else {
            Err(CassetteError::Replay(problems.join("; ")))
        }
    }
}

async fn handle(
    State(shared): State<Arc<Shared>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request = Recorded {
        method: method.to_string(),
        path: uri.path().to_owned(),
        headers: headers
            .iter()
            .map(|(name, value)| {
                let text = String::from_utf8_lossy(value.as_bytes()).into_owned();
                (name.as_str().to_owned(), text)
            })
            .collect(),
        body: serde_json::from_slice(&body)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned())),
    };
    let (response, delay) = shared.decide(&request);
    if let Some(delay) = delay {
        tokio::time::sleep(delay).await;
    }
    response
}

impl Shared {
    /// İsteği kaydeder, sıradaki turla karşılaştırır, yanıtı ve bekleme süresini döndürür.
    fn decide(&self, request: &Recorded) -> (Response, Option<std::time::Duration>) {
        let mut inner = self.lock();
        inner.recorded.push(request.clone());
        let Some(turn) = inner.turns.get(inner.next).cloned() else {
            let number = inner.recorded.len();
            let message = format!("kaset tükendi: beklenmeyen istek #{number}");
            inner.problems.push(message.clone());
            drop(inner);
            return (failure(&message), None);
        };
        inner.next = inner.next.saturating_add(1);
        if let Some(mismatch) = mismatch(&turn, request) {
            let message = format!("tur {}: {mismatch}", turn.turn);
            inner.problems.push(message.clone());
            drop(inner);
            return (failure(&message), None);
        }
        drop(inner);
        let delay = turn.response.delay_ms.map(std::time::Duration::from_millis);
        (replay_response(&turn), delay)
    }
}

fn mismatch(turn: &Turn, request: &Recorded) -> Option<String> {
    let expected = &turn.request;
    if !expected.method.eq_ignore_ascii_case(&request.method) {
        return Some(format!(
            "yöntem uyuşmuyor: beklenen {}, gelen {}",
            expected.method, request.method
        ));
    }
    if expected.path != request.path {
        return Some(format!(
            "yol uyuşmuyor: beklenen {}, gelen {}",
            expected.path, request.path
        ));
    }
    if request.body.is_string() && !expected.body.is_string() {
        return Some("gövde JSON değil".to_owned());
    }
    first_difference(&expected.body, &request.body).map(|diff| format!("gövde uyuşmuyor: {diff}"))
}

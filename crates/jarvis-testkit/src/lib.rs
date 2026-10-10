//! Test sahteleri (Tasarım 0011, ADR 0031). Üretim ikililerine girmez (`production = false`):
//! üretim crate'leri bunu yalnızca `[dev-dependencies]` olarak kullanabilir.
//!
//! Bu sürümde: [`FakeClock`], [`MemorySessions`], [`StubLlm`] ve [`CassetteServer`]
//! (yalnızca tekrar modu). `FakeTool` `jarvis-tools` ile gelir (trait'i orada tanımlanır).

pub mod cassette;
mod cassette_response;
pub mod cassette_server;
pub mod clock;
pub mod sessions;
pub mod stub_llm;

pub use cassette::CassetteError;
pub use cassette_server::{CassetteServer, Recorded};
pub use clock::FakeClock;
pub use sessions::MemorySessions;
pub use stub_llm::{Scripted, StubLlm};

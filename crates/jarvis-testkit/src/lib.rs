//! Test sahteleri (Tasarım 0011, ADR 0031). Üretim ikililerine girmez (`production = false`):
//! üretim crate'leri bunu yalnızca `[dev-dependencies]` olarak kullanabilir.
//!
//! Bu sürümde: [`FakeClock`] ve [`MemorySessions`]. `StubLlm` ve kaset sunucusu
//! `jarvis-provider` ile, `FakeTool` `jarvis-tools` ile gelir (trait'leri orada tanımlanır).

pub mod clock;
pub mod sessions;

pub use clock::FakeClock;
pub use sessions::MemorySessions;

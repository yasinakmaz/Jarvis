//! Olay veriyolu (yayın/abone) ve SSE kaynağı (Tasarım 0005).
//!
//! Her olay önce denetim kaydına yazılır, sonra canlı abonelere yayınlanır: izleyicinin
//! gördüğü her olay denetimde de vardır. Yavaş abone yayıncıyı bloklamaz; kaçırdığını
//! [`Received::Lagged`] ile öğrenir.

pub mod bus;
pub mod event;
pub mod sse;

pub use bus::{AuditError, AuditSink, EventBus, PublishError, Received, Subscription};
pub use event::{Event, EventContext, EventKind, StepPhase, ToolCallSummary};
pub use sse::{LAGGED_EVENT, sse_frame, sse_lagged_frame};

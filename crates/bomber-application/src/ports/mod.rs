//! The ports this layer is driven through.
//!
//! Kept small on purpose. The session is a synchronous state machine that
//! *returns* what should be sent rather than reaching out to send it, so the
//! only thing that genuinely needs to be a port is entropy -- everything else
//! is a return value, which is easier to test than a mock.

mod entropy;

pub use entropy::{FixedEntropy, Entropy};

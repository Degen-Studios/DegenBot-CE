mod handler;
mod processor;

pub use handler::handle;
pub use processor::process_image;

use teloxide::types::{ChatId, MessageId, UserId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;

/// A type alias for a thread-safe, shared map of pending overlays.
///
/// This type represents a collection of pending overlay operations, where each operation
/// is associated with a unique combination of a chat and a user. The map is wrapped in
/// an `Arc<Mutex<>>` to allow safe concurrent access from multiple threads.
///
/// # Type Parameters
///
/// - The key is a tuple of `(ChatId, UserId)`, identifying a unique chat-user combination.
/// - The value is a tuple of `(MessageId, Instant)`, where:
///   - `MessageId` likely refers to the message associated with the overlay.
///   - `Instant` probably represents the time when the overlay operation was initiated or last updated.
///
/// # Usage
///
/// This type is typically used to track and manage ongoing overlay operations across
/// different chats and users in a concurrent environment.
pub type PendingOverlays = Arc<Mutex<HashMap<(ChatId, UserId), (MessageId, Instant)>>>;

#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use teloxide::types::{ChatId, UserId};

/// A queue item that contains a chat ID, user ID, and some data of type `T`.
///
/// This struct is used to represent an item in a queue, which can be enqueued and dequeued.
/// The `_chat_id` and `_user_id` fields are used to identify the context of the queue item,
/// while the `data` field contains the actual data being stored in the queue.
pub struct QueueItem<T> {
    pub _chat_id: ChatId,
    pub _user_id: UserId,
    pub data: T,
}

/// A queue that stores items of type `T`.
///
/// The `Queue` struct is a thread-safe queue that stores items of type `QueueItem<T>`. It provides methods to enqueue, dequeue, and check if the queue is empty.
/// The queue is implemented using an `Arc<Mutex<VecDeque<QueueItem<T>>>>`, which allows for concurrent access and modification of the queue.
pub struct Queue<T> {
    items: Arc<Mutex<VecDeque<QueueItem<T>>>>,
}

/// Implements a thread-safe queue that stores items of type `QueueItem<T>`.
///
/// The `Queue` struct provides methods to enqueue, dequeue, and check if the queue is empty. The queue is implemented using an `Arc<Mutex<VecDeque<QueueItem<T>>>>`, which allows for concurrent access and modification of the queue.
///
/// # Examples
///
/// 
/// use src::utils::queue::{Queue, QueueItem};
/// use tokio::runtime::Runtime;
///
/// let runtime = Runtime::new().unwrap();
/// runtime.block_on(async {
///     let queue = Queue::`<String>`::new();
///     let item = QueueItem {
///         _chat_id: 1,
///         _user_id: 1,
///         data: "hello".to_string(),
///     };
///     queue.enqueue(item).await;
///     let dequeued_item = queue.dequeue().await;
///     assert_eq!(dequeued_item.unwrap().data, "hello");
/// });
/// 
impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            items: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub async fn enqueue(&self, item: QueueItem<T>) {
        let mut queue = self.items.lock().await;
        queue.push_back(item);
    }

    pub async fn dequeue(&self) -> Option<QueueItem<T>> {
        let mut queue = self.items.lock().await;
        queue.pop_front()
    }

    pub async fn is_empty(&self) -> bool {
        let queue = self.items.lock().await;
        queue.is_empty()
    }
}

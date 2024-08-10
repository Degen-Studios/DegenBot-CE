use log::info;
use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId, UserId};
use thiserror::Error;
use axum::{routing::get, Router};
use axum::response::Html;
use shuttle_axum::ShuttleAxum;
use tower_http::trace::TraceLayer;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use tokio::time::Duration;
use shuttle_runtime::SecretStore;

mod config;
mod commands;
mod utils;

use crate::utils::queue::{Queue, QueueItem};
use crate::utils::rate_limiter::RateLimiter;
use crate::utils::cleanup::cleanup_expired_overlays;

#[derive(Debug, Error)]
/// Represents errors that can occur in the Telegram bot application.
///
/// This enum defines two types of errors that can be returned by the bot:
///
/// - `TeloxideError`: Represents errors that occur when making requests to the Telegram API using the Teloxide library.
/// - `IoError`: Represents errors that occur when performing I/O operations, such as reading or writing files.
///
/// These errors are used throughout the application to handle various failure scenarios and provide meaningful error messages to the user or the application's logging system.
enum BotError {
    #[error("Teloxide error: {0}")]
    TeloxideError(#[from] teloxide::RequestError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[shuttle_runtime::main]
/// This is the main entry point for the Telegram bot application. It sets up the necessary components, including the Telegram bot, rate limiter, message queue, and pending overlays, and starts the bot's message handler and cleanup tasks.
///
/// The `main` function is marked with the `#[shuttle_runtime::main]` attribute, which indicates that it is the entry point for the Shuttle runtime. It takes a `SecretStore` parameter, which is used to retrieve the Telegram bot token from the environment.
///
/// The function first initializes the logger, then loads the application configuration. If the Telegram bot is enabled in the configuration, it creates the Telegram bot instance, initializes the necessary data structures (pending overlays, message IDs, rate limiter, and message queue), and sets up the message handler and cleanup tasks.
///
/// The message handler is responsible for processing incoming messages from the Telegram bot, including handling specific commands and enqueuing messages with photos for later processing. The cleanup task periodically checks for and removes expired overlay requests.
///
/// Finally, the function sets up an Axum router with a single route for the root path, which serves a simple HTML response. The router is then returned as the result of the `main` function, which is used by the Shuttle runtime to deploy the application.
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> ShuttleAxum {
    let _ = pretty_env_logger::try_init();
    info!("Starting bot...");

    let config = config::load_config();

    if config.telegram.enabled {
        let bot_token = secrets.get("TELEGRAM_BOT_TOKEN")
            .expect("TELEGRAM_BOT_TOKEN secret not found");
        let bot = Bot::new(&bot_token);

        let pending_overlays: commands::PendingOverlays = Arc::new(Mutex::new(HashMap::new()));
        let message_ids: Arc<Mutex<HashMap<(ChatId, UserId), MessageId>>> = Arc::new(Mutex::new(HashMap::new()));
        let rate_limiter = Arc::new(RateLimiter::new(5, Duration::from_secs(60))); // 5 requests per minute
        let message_queue = Arc::new(Queue::<Message>::new());

        let handler_pending_overlays = Arc::clone(&pending_overlays);
        let handler_message_ids = Arc::clone(&message_ids);
        let handler_rate_limiter = Arc::clone(&rate_limiter);
        let handler_message_queue = Arc::clone(&message_queue);

        let handler = dptree::entry()
            .branch(Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
                let pending_overlays = Arc::clone(&handler_pending_overlays);
                let message_ids = Arc::clone(&handler_message_ids);
                let rate_limiter = Arc::clone(&handler_rate_limiter);
                let message_queue = Arc::clone(&handler_message_queue);
                async move {
                    message_handler(bot, msg, pending_overlays, message_ids, rate_limiter, message_queue).await
                }
            }));

        tokio::spawn(async move {
            Dispatcher::builder(bot, handler)
                .enable_ctrlc_handler()
                .build()
                .dispatch()
                .await;
        });

        // Spawn a task to clean up expired overlay requests
        let cleanup_bot = Bot::new(&bot_token);
        let cleanup_pending_overlays = Arc::clone(&pending_overlays);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await; // Run every minute
                cleanup_expired_overlays(cleanup_bot.clone(), cleanup_pending_overlays.clone()).await;
            }
        });

        // Spawn a task to process the message queue
        let queue_bot = Bot::new(&bot_token);
        let queue_pending_overlays = Arc::clone(&pending_overlays);
        let queue_message_queue = Arc::clone(&message_queue);
        tokio::spawn(async move {
            process_queue(queue_bot, queue_pending_overlays, queue_message_queue).await;
        });
    } else {
        info!("Telegram bot is disabled in config.");
    }

    let router = Router::new()
        .route("/", get(index))
        .layer(TraceLayer::new_for_http());

    Ok(router.into())
}

/// Handles incoming messages for the Telegram bot.
///
/// This function is called whenever a new message is received by the bot. It checks the message text and
/// performs the appropriate action, such as starting the bot or processing an image overlay request.
/// If the message contains a photo, it is enqueued in the `message_queue` for later processing.
/// The function also checks the rate limit for the user and sends a message if they are sending commands too quickly.
async fn message_handler(
    bot: Bot,
    msg: Message,
    pending_overlays: commands::PendingOverlays,
    message_ids: Arc<Mutex<HashMap<(ChatId, UserId), MessageId>>>,
    rate_limiter: Arc<RateLimiter>,
    message_queue: Arc<Queue<Message>>,
) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        if text.starts_with("/start") {
            commands::start::start(bot.clone(), msg).await?;
        } else if text.starts_with("/degenme") {
            let chat_id = msg.chat.id;
            let user_id = msg.from().map(|user| user.id).unwrap_or(UserId(0));
            
            if rate_limiter.check_rate_limit(&format!("{}:{}", chat_id, user_id)).await {
                commands::overlay::handle(bot.clone(), msg, pending_overlays.clone(), message_ids.clone(), rate_limiter.clone()).await;
            } else {
                bot.send_message(chat_id, "You're sending commands too quickly. Please wait a moment before trying again.").await?;
            }
        }
    } else if msg.photo().is_some() {
        message_queue.enqueue(QueueItem { _chat_id: msg.chat.id, _user_id: msg.from().map(|user| user.id).unwrap_or(UserId(0)), data: msg }).await;
    }

    Ok(())
}

/// Processes the message queue, handling incoming messages for the Telegram bot.
///
/// This function runs in a loop, continuously dequeuing messages from the `message_queue` and processing them.
/// For each message, it calls the `commands::overlay::process_image` function to handle the message.
/// If an error occurs while processing a message, it is logged using `log::error`.
/// The function also includes a short delay of 100 milliseconds between each iteration of the loop.
async fn process_queue(bot: Bot, pending_overlays: commands::PendingOverlays, message_queue: Arc<Queue<Message>>) {
    loop {
        if let Some(item) = message_queue.dequeue().await {
            commands::overlay::process_image(bot.clone(), item.data, pending_overlays.clone()).await.unwrap_or_else(|e| {
                log::error!("Error processing image: {:?}", e);
            });
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// This function returns an HTML response that redirects the user to the "<https://degenstudios.media>" URL.
/// The response includes a meta refresh tag that automatically redirects the user, and also includes a link
/// that the user can click if they are not automatically redirected.
async fn index() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta http-equiv="refresh" content="0; URL=https://degenstudios.media">
    <title>Redirecting...</title>
</head>
<body>
    <p>If you are not redirected, <a href="https://degenstudios.media">click here</a>.</p>
</body>
</html>"#)
}

use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId, UserId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;
use log::{info, error};
use crate::commands::CommandResponse;
use crate::utils::rate_limiter::RateLimiter;
use super::PendingOverlays;

/// A struct that handles the command processing for the overlay feature.
///
/// This struct contains the necessary dependencies to handle the overlay command, including the bot instance,
/// the pending overlays, the message IDs, and the rate limiter.
pub struct CommandHandler {
    bot: Bot,
    pending_overlays: PendingOverlays,
    message_ids: Arc<Mutex<HashMap<(ChatId, UserId), MessageId>>>,
    rate_limiter: Arc<RateLimiter>,
}

/// Handles the command processing for the overlay feature.
///
/// This implementation provides the necessary functionality to handle the overlay command, including:
/// - Checking the rate limit for the user and chat
/// - Sending a reply message with instructions for the user
/// - Managing the pending overlays for each user and chat
///
/// The `handle` method is the main entry point for processing the overlay command.
impl CommandHandler {
    pub fn new(bot: Bot, pending_overlays: PendingOverlays, message_ids: Arc<Mutex<HashMap<(ChatId, UserId), MessageId>>>, rate_limiter: Arc<RateLimiter>) -> Self {
        CommandHandler {
            bot,
            pending_overlays,
            message_ids,
            rate_limiter,
        }
    }

    /// Handles the "overlay" command, which allows users to request an image overlay.
    ///
    /// This function is responsible for processing the "overlay" command, which allows users to request an image overlay. It checks the rate limit, manages the pending overlay requests, and sends a reply message to the user with instructions on how to submit an image for the overlay.
    ///
    /// # Arguments
    /// * `bot` - The Telegram bot instance.
    /// * `msg` - The incoming message that triggered the "overlay" command.
    /// * `pending_overlays` - A shared mutex-protected map of pending overlay requests.
    /// * `message_ids` - A shared mutex-protected map of message IDs for pending overlay requests.
    /// * `rate_limiter` - A rate limiter to prevent users from sending commands too quickly.
    ///
    /// # Returns
    /// A `CommandResponse` that represents the result of handling the "overlay" command.
    pub fn handle<'a>(
        bot: Bot,
        msg: Message,
        pending_overlays: PendingOverlays,
        _message_ids: Arc<Mutex<HashMap<(ChatId, UserId), MessageId>>>,
        rate_limiter: Arc<RateLimiter>
    ) -> CommandResponse<'a> {
        Box::pin(async move {
            info!("Entering overlay handle function");
            let user_id = msg.from().map(|user| user.id);
            let chat_id = msg.chat.id;
            info!("User ID: {:?}, Chat ID: {}", user_id, chat_id);

            let username = msg.from()
                .and_then(|user| user.username.as_ref())
                .map(|username| format!("@{}", username))
                .unwrap_or_else(|| "there".to_string());

            info!("Username: {}", username);

            // Check rate limit
            if !rate_limiter.check_rate_limit(&format!("{}:{}", chat_id, user_id.unwrap_or(UserId(0)))).await {
                if let Err(e) = bot.send_message(chat_id, "You're sending commands too quickly. Please wait a moment before trying again.").await {
                    error!("Failed to send rate limit message: {}", e);
                }
                return;
            }

            let mut overlays = pending_overlays.lock().await;
            let reply_text = if let Some(user_id) = user_id {
                if overlays.contains_key(&(chat_id, user_id)) {
                    format!("Previous request cancelled. Hey, {}! Please reply within 3 minutes to this message with an image to see the Degen Point of View!", username)
                } else {
                    format!("Hey, {}! Please reply within 3 minutes to this message with an image to see the Degen Point of View!", username)
                }
            } else {
                format!("Hey, {}! Please reply within 3 minutes to this message with an image to see the Degen Point of View!", username)
            };

            info!("Sending reply: {}", reply_text);

            let reply = bot.send_message(msg.chat.id, reply_text).await;
            match reply {
                Ok(sent) => {
                    info!("Reply sent successfully. Message ID: {}", sent.id);
                    if let Some(user_id) = user_id {
                        // Remove any existing pending overlay for this user
                        overlays.remove(&(chat_id, user_id));
                        // Insert new pending overlay with current timestamp
                        overlays.insert((chat_id, user_id), (sent.id, Instant::now()));
                        info!("Inserted pending overlay request. Chat ID: {}, User ID: {}, Message ID: {}", chat_id, user_id, sent.id);
                        info!("Current pending overlays: {:?}", overlays);
                    } else {
                        error!("Failed to get user ID for pending overlay request");
                    }
                },
                Err(e) => {
                    error!("Failed to send message: {}", e);
                }
            }
            info!("Exiting overlay handle function");
        })
    }
}

/// Handles the "overlay" command, which allows users to request an image overlay.
///
/// This function is responsible for processing the "overlay" command, which allows users to request an image overlay. It checks the rate limit, manages the pending overlay requests, and sends a reply message to the user with instructions on how to submit an image for the overlay.
///
/// # Arguments
/// * `bot` - The Telegram bot instance.
/// * `msg` - The incoming message that triggered the "overlay" command.
/// * `pending_overlays` - A shared mutex-protected map of pending overlay requests.
/// * `message_ids` - A shared mutex-protected map of message IDs for pending overlay requests.
/// * `rate_limiter` - A rate limiter to prevent users from sending commands too quickly.
///
/// # Returns
/// A `CommandResponse` that represents the result of handling the "overlay" command.
pub fn handle<'a>(
    bot: Bot,
    msg: Message,
    pending_overlays: PendingOverlays,
    message_ids: Arc<Mutex<HashMap<(ChatId, UserId), MessageId>>>,
    rate_limiter: Arc<RateLimiter>
) -> CommandResponse<'a> {
    CommandHandler::handle(bot, msg, pending_overlays, message_ids, rate_limiter)
}

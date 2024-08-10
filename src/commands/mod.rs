#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{Message, MessageId, ChatId, UserId};
use tokio::sync::Mutex;
use log::info;
use tokio::time::Instant;
use std::pin::Pin;
use std::future::Future;

pub mod overlay;
pub mod start;

use crate::utils::rate_limiter::RateLimiter;

/// A type alias for a Future that represents a command response.
/// The Future must be pinned, boxed, and implement Send to be used
/// as a command response.
pub type CommandResponse<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>>;
/// A shared state for tracking pending overlay operations.
/// This type is an `Arc<Mutex<HashMap<(ChatId, UserId), (MessageId, Instant)>>>`,
/// which allows multiple parts of the application to safely access and modify
/// the pending overlay state concurrently.
pub type PendingOverlays = Arc<Mutex<HashMap<(ChatId, UserId), (MessageId, Instant)>>>;

#[derive(Clone)]
/// The `CommandHandler` struct is responsible for registering and executing
/// the various commands supported by the Telegram bot. It maintains a map of
/// command names to their corresponding handler functions, and provides
/// methods to register new commands and execute them.
///
/// The `CommandHandler` struct holds the necessary dependencies for managing
/// the bot's commands, such as the bot instance, shared state for pending
/// overlays and message IDs, and a rate limiter.
pub struct CommandHandler {
    commands: Arc<HashMap<String, Arc<dyn Fn(Bot, Message, Arc<Mutex<HashMap<(ChatId, UserId), (MessageId, Instant)>>>, Arc<Mutex<HashMap<(ChatId, UserId), MessageId>>>, Arc<RateLimiter>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync>>>,
    bot: Bot,
    pending_overlays: PendingOverlays,
    message_ids: Arc<Mutex<HashMap<(ChatId, UserId), MessageId>>>,
    rate_limiter: Arc<RateLimiter>,
}

/// The `CommandHandler` struct is responsible for registering and executing
/// the various commands supported by the Telegram bot. It maintains a map of
/// command names to their corresponding handler functions, and provides
/// methods to register new commands and execute them.
///
/// The `CommandHandler` struct holds the necessary dependencies for managing
/// the bot's commands, such as the bot instance, shared state for pending
/// overlays and message IDs, and a rate limiter.
impl CommandHandler {
    /// Constructs a new `CommandHandler` instance with the provided dependencies.
    ///
    /// The `CommandHandler` is responsible for managing the bot's commands, including
    /// registering new commands and executing them. It holds references to the bot
    /// instance, shared state for pending overlays and message IDs, and a rate limiter.
    ///
    /// # Arguments
    /// - `bot`: The `Bot` instance for the Telegram bot.
    /// - `pending_overlays`: A shared state for tracking pending overlay operations.
    /// - `message_ids`: A shared state for tracking message IDs.
    /// - `rate_limiter`: A rate limiter for limiting the number of requests per minute.
    ///
    /// # Returns
    /// A new `CommandHandler` instance with the provided dependencies.
    pub fn new(
        bot: Bot,
        pending_overlays: PendingOverlays,
        message_ids: Arc<Mutex<HashMap<(ChatId, UserId), MessageId>>>,
        rate_limiter: Arc<RateLimiter>
    ) -> Self {
        CommandHandler {
            commands: Arc::new(HashMap::new()),
            bot,
            pending_overlays,
            message_ids,
            rate_limiter,
        }
    }

    /// Registers the "degenme" and "start" commands with the `CommandHandler`.
    ///
    /// The "degenme" command is registered with the `overlay::handle` function as its handler.
    /// The "start" command is registered with an anonymous function that calls the `start::start` function.
    fn register_commands(&mut self) {
        self.register_command("degenme", Arc::new(overlay::handle));
        self.register_command("start", Arc::new(|bot, msg, _pending_overlays, _message_ids, _rate_limiter| -> CommandResponse {
            Box::pin(async move {
                if let Err(e) = start::start(bot, msg).await {
                    log::error!("Error in start command: {:?}", e);
                }
            })
        }));        
    }

    /// Registers a new command with the `CommandHandler`.
    ///
    /// This method adds a new command to the `CommandHandler`'s internal command registry.
    /// The `name` parameter specifies the command name, and the `command` parameter is a
    /// closure that will be executed when the command is invoked.
    ///
    /// The closure must have the following signature:
    /// `Fn(Bot, Message, Arc<Mutex<HashMap<(ChatId, UserId), (MessageId, Instant)>>>, Arc<Mutex<HashMap<(ChatId, UserId), MessageId>>>, Arc<RateLimiter>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>`
    ///
    /// This allows the command handler to pass the necessary dependencies to the command
    /// implementation, such as the bot instance, shared state for pending overlays and
    /// message IDs, and a rate limiter.
    ///
    /// # Arguments
    /// - `name`: The name of the command to register.
    /// - `command`: The command implementation as a closure.
    fn register_command<F>(&mut self, name: &str, command: Arc<F>)
    where
        F: Fn(Bot, Message, Arc<Mutex<HashMap<(ChatId, UserId), (MessageId, Instant)>>>, Arc<Mutex<HashMap<(ChatId, UserId), MessageId>>>, Arc<RateLimiter>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync + 'static,
    {
        Arc::get_mut(&mut self.commands).unwrap().insert(name.to_string(), command);
    }

}

/// Handles an incoming message from the bot, processing any commands or photos.
///
/// This function is responsible for parsing incoming messages, identifying any commands,
/// and executing the corresponding command handler. If the message contains a photo,
/// it is passed to the `overlay::process_image` function for further processing.
///
/// The function creates a new `CommandHandler` instance, which manages the registration
/// and execution of commands. The `CommandHandler` is initialized with the necessary
/// dependencies, such as the bot instance, pending overlays, message IDs, and a rate
/// limiter.
///
/// # Arguments
/// - `bot`: The bot instance to use for processing the message.
/// - `msg`: The incoming message to be handled.
/// - `pending_overlays`: A shared state for tracking pending overlays.
/// - `message_ids`: A shared state for tracking message IDs.
///
/// # Returns
/// A `ResponseResult` indicating the success or failure of the message handling.
pub async fn handle_message(bot: Bot, msg: Message, pending_overlays: PendingOverlays, message_ids: Arc<Mutex<HashMap<(ChatId, UserId), MessageId>>>) -> ResponseResult<()> {
    info!("Entering handle_message function");
    let rate_limiter = Arc::new(RateLimiter::new(5, std::time::Duration::from_secs(60))); // 5 requests per minute
    let handler = CommandHandler::new(bot.clone(), pending_overlays.clone(), message_ids.clone(), rate_limiter.clone());
    info!("CommandHandler created");

    if let Some(text) = msg.text() {
        info!("Received text message: {}", text);
        let parts: Vec<&str> = text.splitn(2, ' ').collect();
        if let Some(command_with_username) = parts.first() {
            if command_with_username.starts_with('/') {
                let command_parts: Vec<&str> = command_with_username.splitn(2, '@').collect();
                let command_name = &command_parts[0][1..];
                info!("Processing command: {}", command_name);
                if let Some(command_handler) = handler.commands.get(command_name) {
                    info!("Executing command handler for: {}", command_name);
                    command_handler(bot.clone(), msg.clone(), handler.pending_overlays.clone(), handler.message_ids.clone(), handler.rate_limiter.clone()).await;
                } else {
                    // Do nothing,
                    // Will cause it to respond to
                    // Commands for other bots otherwise.
                }
            }
        }
    } else if msg.photo().is_some() {
        info!("Received photo message");
        overlay::process_image(bot.clone(), msg, handler.pending_overlays.clone()).await?;
    } else {
        info!("Received message without text or photo");
    }

    info!("Exiting handle_message function");
    Ok(())
}

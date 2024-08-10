use teloxide::prelude::*;
use teloxide::types::InputFile;
use opencv::{core, imgcodecs};
use opencv::prelude::*;
use reqwest;
use std::path::Path;
use log::{info, error, warn};
use tokio::time::{sleep, Duration};

use crate::utils::queue::{Queue, QueueItem};
use crate::utils::image_utils::overlay_image;
use super::PendingOverlays;
use crate::utils::cleanup::OVERLAY_EXPIRATION;

/// The maximum number of retries allowed when processing an image overlay request.
const MAX_RETRIES: usize = 3;

/// The ImageProcessor struct is responsible for managing the queue of image overlay requests,
/// processing them, and interacting with the Telegram bot and the pending overlays.
/// It has a queue to store the incoming overlay requests, a reference to the Telegram bot,
/// and a reference to the pending overlays.
pub struct ImageProcessor {
    queue: Queue<Message>,
    bot: Bot,
    pending_overlays: PendingOverlays,
}

/// The `process_image` function is responsible for processing an image overlay request received from a Telegram message.
/// It creates a new `ImageProcessor` instance, enqueues the message, and then processes the queue.
/// The function returns a `ResponseResult<()>` indicating the success or failure of the operation.
impl ImageProcessor {
    pub fn new(bot: Bot, pending_overlays: PendingOverlays) -> Self {
        ImageProcessor {
            queue: Queue::new(),
            bot,
            pending_overlays,
        }
    }

    /// Enqueues a message in the queue for processing.
    ///
    /// This method creates a new `QueueItem` from the provided `Message` and enqueues it in the `queue`.
    /// The `_chat_id` and `_user_id` fields of the `QueueItem` are set based on the information in the `Message`.
    /// The `data` field of the `QueueItem` is set to the `Message` itself.
    pub async fn enqueue(&self, msg: Message) {
        let item = QueueItem {
            _chat_id: msg.chat.id,
            _user_id: msg.from().map(|user| user.id).unwrap_or(UserId(0)),
            data: msg,
        };
        self.queue.enqueue(item).await;
    }

    /// Processes the queue of image overlay requests.
    ///
    /// This method continuously dequeues items from the `queue` and processes the associated `Message` objects.
    /// For each message, it calls the `process_image` method to handle the image overlay request.
    /// If an error occurs during processing, it logs the error and continues to the next item in the queue.
    pub async fn process_queue(&self) {
        while let Some(item) = self.queue.dequeue().await {
            self.process_image(item.data).await.unwrap_or_else(|e| {
                error!("Error processing image: {:?}", e);
            });
        }
    }

    /// Processes an image overlay request received from a Telegram message.
    ///
    /// This function creates a new `ImageProcessor` instance, enqueues the message, and then processes the queue.
    /// It returns a `ResponseResult<()>` indicating the success or failure of the operation.
    ///
    /// # Arguments
    /// * `bot` - A reference to the Telegram bot instance.
    /// * `msg` - The Telegram message containing the image overlay request.
    /// * `pending_overlays` - A reference to the pending overlays.
    ///
    /// # Returns
    /// A `ResponseResult<()>` indicating the success or failure of the operation.
    async fn process_image(&self, msg: Message) -> ResponseResult<()> {
        info!("Entering process_image function");
        let user_id = msg.from().map(|user| user.id);
        let mut overlays = self.pending_overlays.lock().await;
        info!("Acquired lock on pending_overlays");

        if let (Some(user_id), Some(reply_to)) = (user_id, msg.reply_to_message()) {
            info!("User ID: {:?}, Reply to message ID: {}", user_id, reply_to.id);
            if let Some(&(original_msg_id, request_time)) = overlays.get(&(msg.chat.id, user_id)) {
                info!("Found original message ID in pending_overlays: {}", original_msg_id);
                info!("Comparing original_msg_id: {} with reply_to.id: {}", original_msg_id, reply_to.id);
                if original_msg_id == reply_to.id {
                    if request_time.elapsed() > OVERLAY_EXPIRATION {
                        info!("Overlay request has expired");
                        overlays.remove(&(msg.chat.id, user_id));
                        self.bot.send_message(msg.chat.id, "Your overlay request has expired. Please use the /degenme command again.").await?;
                        return Ok(());
                    }
                    info!("Reply matches the original overlay request");
                    overlays.remove(&(msg.chat.id, user_id));
                    info!("Removed overlay request from pending_overlays");
                    drop(overlays);
                    info!("Released lock on pending_overlays");

                    if let Some(photo) = msg.photo().and_then(|photos| photos.last()) {
                        info!("Found photo in message");
                        let username = msg.from()
                            .and_then(|user| user.username.as_ref())
                            .map(|username| format!("@{}", username))
                            .unwrap_or_else(|| "Anonymous".to_string());

                        info!("Processing image for user: {}", username);
                        let processing_msg = self.bot.send_message(msg.chat.id, format!("Making {} a degen... Please wait...", username)).await?;
                        info!("Sent processing message");

                        info!("Fetching file from Telegram");
                        let file = match self.bot.get_file(&photo.file.id).await {
                            Ok(file) => file,
                            Err(e) => {
                                error!("Failed to get file: {}", e);
                                self.bot.delete_message(msg.chat.id, processing_msg.id).await?;
                                self.bot.send_message(msg.chat.id, "Failed to process your image. Please try again.").await?;
                                return Ok(());
                            }
                        };

                        info!("Downloading image");
                        let url = format!("https://api.telegram.org/file/bot{}/{}", self.bot.token(), file.path);
                        let response = match reqwest::get(&url).await {
                            Ok(response) => response,
                            Err(e) => {
                                error!("Failed to download image: {}", e);
                                self.bot.delete_message(msg.chat.id, processing_msg.id).await?;
                                self.bot.send_message(msg.chat.id, "Failed to download your image. Please try again.").await?;
                                return Ok(());
                            }
                        };

                        info!("Reading image data");
                        let image_data = match response.bytes().await {
                            Ok(data) => data,
                            Err(e) => {
                                error!("Failed to read image data: {}", e);
                                self.bot.delete_message(msg.chat.id, processing_msg.id).await?;
                                self.bot.send_message(msg.chat.id, "Failed to read your image. Please try again.").await?;
                                return Ok(());
                            }
                        };

                        info!("Decoding image");
                        let img = match imgcodecs::imdecode(&core::Vector::from_slice(&image_data), imgcodecs::IMREAD_COLOR) {
                            Ok(img) => img,
                            Err(e) => {
                                error!("Failed to decode image: {}", e);
                                self.bot.delete_message(msg.chat.id, processing_msg.id).await?;
                                self.bot.send_message(msg.chat.id, "Failed to decode your image. Please try again.").await?;
                                return Ok(());
                            }
                        };

                        const ASPECT_RATIO_TOLERANCE: f32 = 0.05; // 5% tolerance

                        let aspect_ratio = img.rows() as f32 / img.cols() as f32;
                        let is_portrait = aspect_ratio > (1.0 + ASPECT_RATIO_TOLERANCE);
                        let overlay_path = if is_portrait {
                            Path::new("img/hands_portrait.png")
                        } else {
                            Path::new("img/hands_landscape.png")
                        };
                        info!("Using overlay: {:?}", overlay_path);

                        info!("Reading overlay image");
                        let overlay = match imgcodecs::imread(overlay_path.to_str().unwrap(), imgcodecs::IMREAD_UNCHANGED) {
                            Ok(overlay) => overlay,
                            Err(e) => {
                                error!("Failed to read overlay image: {}", e);
                                self.bot.delete_message(msg.chat.id, processing_msg.id).await?;
                                self.bot.send_message(msg.chat.id, "Failed to process overlay. Please try again later.").await?;
                                return Ok(());
                            }
                        };

                        info!("Starting image overlay process");
                        let mut retry_count = 0;
                        let mut previous_result: Option<Mat> = None;
                        let result = loop {
                            match overlay_image(&img, &overlay, previous_result.as_ref()) {
                                Ok(result) => break result,
                                Err(e) if retry_count < MAX_RETRIES => {
                                    warn!("Error in overlay_image, retrying (attempt {}): {}", retry_count + 1, e);
                                    retry_count += 1;
                                    sleep(Duration::from_millis(500)).await;
                                    if let Some(prev) = previous_result {
                                        previous_result = Some(prev);
                                    }
                                },
                                Err(e) => {
                                    error!("Failed to overlay image after {} retries: {}", MAX_RETRIES, e);
                                    self.bot.delete_message(msg.chat.id, processing_msg.id).await?;
                                    self.bot.send_message(msg.chat.id, "Failed to process your image. Please try again later.").await?;
                                    return Ok(());
                                }
                            }
                        };

                        info!("Encoding result image");
                        let mut opencv_buffer = core::Vector::new();
                        if let Err(e) = imgcodecs::imencode(".png", &result, &mut opencv_buffer, &core::Vector::new()) {
                            error!("Failed to encode result image: {}", e);
                            self.bot.delete_message(msg.chat.id, processing_msg.id).await?;
                            self.bot.send_message(msg.chat.id, "Failed to process your image. Please try again.").await?;
                            return Ok(());
                        }
                        let buffer = opencv_buffer.to_vec();
                        
                        info!("Sending processed image");
                        
                        let caption = format!("Here you go {}, you degen.", username);
                        let sent_photo = self.bot.send_photo(msg.chat.id, InputFile::memory(buffer).file_name("overlay.png"))
                            .caption(caption)
                            .await?;
                    
                        info!("Image sent successfully with caption");

                        info!("Sent photo message ID: {}", sent_photo.id);

                        // Now delete the processing message
                        if let Err(e) = self.bot.delete_message(msg.chat.id, processing_msg.id).await {
                            error!("Failed to delete processing message: {}", e);
                        }
                    } else {
                        warn!("No photo found in the message");
                        self.bot.send_message(msg.chat.id, "Please reply with an image to degen.").await?;
                    }
                } else {
                    info!("Reply does not match the original overlay request. Expected: {}, Got: {}", original_msg_id, reply_to.id);
                }
            } else {
                info!("No pending overlay request found for user ID: {:?} in chat ID: {}", user_id, msg.chat.id);
            }
        } else {
            info!("Message is not a reply or user ID is missing. User ID: {:?}, Is reply: {}", user_id, msg.reply_to_message().is_some());
        }

        info!("Exiting process_image function");
        Ok(())
    }
}

/// Processes an image message received by the bot.
///
/// This function is responsible for handling the processing of an image message received by the bot. It enqueues the message for processing and then processes the queue. If the processing is successful, it sends the processed image back to the user with a caption. If there are any errors during the processing, it sends an error message to the user.
///
/// # Arguments
/// * `bot` - The Telegram bot instance.
/// * `msg` - The message containing the image to be processed.
/// * `pending_overlays` - The pending overlays for the user.
///
/// # Returns
/// A `ResponseResult<()>` indicating the success or failure of the operation.
pub async fn process_image(bot: Bot, msg: Message, pending_overlays: PendingOverlays) -> ResponseResult<()> {
    let processor = ImageProcessor::new(bot, pending_overlays);
    processor.enqueue(msg).await;
    processor.process_queue().await;
    Ok(())
}

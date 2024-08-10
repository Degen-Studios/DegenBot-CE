use teloxide::prelude::*;
use log::{info, error};
use tokio::time::{ Duration, Instant };

use crate::commands::overlay::PendingOverlays;

/// The duration after which an overlay request is considered expired and should be removed.
/// This is set to 3 minutes.
pub const OVERLAY_EXPIRATION: Duration = Duration::from_secs(180); // 3 minutes

/// Cleans up expired overlay requests by removing them from the `PendingOverlays` map and sending an expiry message to the user.
///
/// This function is called periodically to maintain the `PendingOverlays` map and ensure that expired overlay requests are removed.
/// It iterates through the map, finds any requests that have been pending for longer than `OVERLAY_EXPIRATION` (3 minutes),
/// removes them from the map, and sends an expiry message to the user.
///
/// # Arguments
/// * `bot` - The `Bot` instance used to interact with the Telegram API.
/// * `pending_overlays` - The `PendingOverlays` map that stores the pending overlay requests.
pub async fn cleanup_expired_overlays(bot: Bot, pending_overlays: PendingOverlays) {
    let mut overlays = pending_overlays.lock().await;
    let now = Instant::now();
    let expired: Vec<_> = overlays
        .iter()
        .filter(|(_, (_, time))| now.duration_since(*time) > OVERLAY_EXPIRATION)
        .map(|((chat_id, user_id), _)| (*chat_id, *user_id))
        .collect();

    for (chat_id, user_id) in expired {
        if let Some((msg_id, _)) = overlays.remove(&(chat_id, user_id)) {
            info!("Removing expired overlay request for Chat ID: {}, User ID: {}", chat_id, user_id);
            if let Ok(chat_member) = bot.get_chat_member(chat_id, user_id).await {
                let username = chat_member.user.username.unwrap_or_else(|| "Degen".to_string());
                let expiry_message = format!("{}, you degen, you forgot to send me a picture! Please run /degenme again to send an image.", username);
                if let Err(e) = bot.send_message(chat_id, expiry_message).await {
                    error!("Failed to send expiry message: {}", e);
                }
            }
            if let Err(e) = bot.delete_message(chat_id, msg_id).await {
                error!("Failed to delete expired overlay message: {}", e);
            }
        }
    }
}

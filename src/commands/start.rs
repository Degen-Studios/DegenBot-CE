use teloxide::prelude::*;

/// Starts the DegenMe bot and sends a welcome message to the user.
///
/// This function is called when the `/start` command is received by the bot. It sends a welcome message to the user
/// with instructions on how to use the bot.
///
/// # Arguments
/// * `bot` - The Teloxide bot instance.
/// * `msg` - The message that triggered the command.
///
/// # Returns
/// A `ResponseResult` indicating the success or failure of the operation.
pub async fn start(bot: Bot, msg: Message) -> ResponseResult<()> {
    let response = "Welcome to the Degen POV bot! Use /degenme to create an overlay in any channel, group, or DM I am in!";
    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}

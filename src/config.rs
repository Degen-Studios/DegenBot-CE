use serde::Deserialize;
use std::fs;

/// The main configuration for the application.
///
/// This struct contains the configuration for various components of the application,
/// such as the Telegram integration.
#[derive(Deserialize)]
pub struct Config {
    pub telegram: TelegramConfig,
}

/// Represents the configuration for the Telegram integration.
///
/// This struct contains the settings for the Telegram bot, such as whether it is enabled or not.

#[derive(Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
}

/// Loads the application's configuration from a TOML file located at "config.toml".
///
/// This function reads the contents of the "config.toml" file, parses it using the `toml` crate,
/// and returns the resulting `Config` struct. If there is an error reading or parsing the
/// configuration file, the function will panic with an appropriate error message.
pub fn load_config() -> Config {
    let config_content = fs::read_to_string("config.toml").expect("Failed to read config file");
    toml::from_str(&config_content).expect("Failed to parse config file")
}

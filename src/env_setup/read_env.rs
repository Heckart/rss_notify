use log::{debug, trace};
use serde::Deserialize;
use std::env::var;
use std::fs::read_to_string;
use toml::from_str;

#[derive(Deserialize)]
pub struct FeedInfo {
    pub url: String,
    pub prefix: String,
    pub blacklist: Vec<String>,
}

#[derive(Deserialize)]
pub struct FeedConfig {
    pub feeds: Vec<FeedInfo>,
}

/// **Purpose**:    Grabs the list of rss feeds in the file represented by an env var
/// **Parameters**: A &str representing the name of an environment variable holding a feed list file
/// **Returns**:    A vec<String> of all feeds urls in the file
/// **Panics**:     If the file cannot be opened or read
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn get_feed_config(feed_config_var: &str) -> FeedConfig {
    trace!("Inside get_feed_config with feed_config_var as {feed_config_var}.");

    let config_file_contents: String = match read_to_string(source_env_var(feed_config_var)) {
        Ok(contents) => {
            trace!("Read feed config contents.");
            contents
        }
        Err(err) => {
            panic!("Could not convert file contents to string! {err}.");
        }
    };

    let structured_config: FeedConfig = match from_str(&config_file_contents) {
        Ok(contents) => {
            trace!("Converted feed config string to FeedConfig structure.");
            contents
        }
        Err(err) => {
            panic!("Could not convert feed config string to FeedConfig! {err}.");
        }
    };

    structured_config
}

/// **Purpose**:    Grabs the contents of an envrionment variable
/// **Parameters**: A &str representing the name of an environment variable
/// **Returns**:    A String containing the contents of the passed env var
/// **Panics**:     If the env var cannot be sourced
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn source_env_var(env_var: &str) -> String {
    trace!("Inside source_env_var with env_var as {env_var}.");
    let env_var_content: String = match var(env_var) {
        Ok(var) => {
            debug!("Sourced {env_var} as {var}.");
            var
        }
        Err(err) => {
            panic!("Could not source env variable! {err}");
        }
    };

    env_var_content
}

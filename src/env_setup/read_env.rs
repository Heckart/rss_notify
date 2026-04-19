use log::{debug, trace};
use std::env::var;
use std::fs::File;
use std::io::Read;

/// **Purpose**:    Grabs the list of rss feeds in the file represented by an env var
/// **Parameters**: A &str representing the name of an environment variable holding a feed list file
/// **Returns**:    A vec<String> of all feeds urls in the file
/// **Panics**:     If the file cannot be opened or read
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn get_feed_list(feed_env_var: &str) -> Vec<String> {
    trace!(
        "Inside get_feed_list with feed_env_var as {}.",
        feed_env_var
    );
    // open the feed url file, specified by the env variable
    let mut feed_url_file: File = match File::open(source_env_var(feed_env_var)) {
        Ok(file_name) => {
            trace!("Opened feed url file.");
            file_name
        }
        Err(err) => {
            panic!("Could not open feed url file! {}.", err)
        }
    };

    // grab the txt contents of the feed url file
    let mut url_file_contents: String = String::new();
    match feed_url_file.read_to_string(&mut url_file_contents) {
        Ok(_) => {
            trace!("Read feed url file contents.");
        }
        Err(err) => {
            panic!("Could not convert file contents to string! {}", err)
        }
    };

    // construct a vector of all the urls
    let url_list: Vec<String> = url_file_contents
        .trim()
        .lines()
        .map(String::from)
        .collect::<Vec<String>>();

    url_list
}

/// **Purpose**:    Grabs the contents of an envrionment variable
/// **Parameters**: A &str representing the name of an environment variable
/// **Returns**:    A String containing the contents of the passed env var
/// **Panics**:     If the env var cannot be sourced
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn source_env_var(env_var: &str) -> String {
    trace!("Inside source_env_var with env_var as {}.", env_var);
    let env_var_content: String = match var(env_var) {
        Ok(var) => {
            debug!("Sourced {} as {}.", env_var, var);
            var
        }
        Err(err) => {
            panic!("Could not source env variable! {}", err);
        }
    };

    env_var_content
}

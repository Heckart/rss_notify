use crate::env_setup::source_env_var;
use log::{debug, trace};
use reqwest::{Error, blocking::Client, blocking::Response};
use rss::Item;

/// **Purpose**:    Sends a POST to ntfy to make pushes with article title and link of new rss content
/// **Parameters**: A &Vec<rss:Item> of new rss content
/// **Ok Return**:  A Vec<reqwest::blocking::Response> of all responses received from ntfy
/// **Err Return**: A Vec<reqwest::Error> from any unsuccessful POSTs
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn send_new_item_notification(items: &Vec<Item>) -> Vec<Result<Response, Error>> {
    trace!(
        "Inside send_new_item_notification with {} items.",
        items.len()
    );
    let ntfy_topic: String = source_env_var("NTFY_TOPIC");

    let client: Client = Client::new();
    let mut responses: Vec<Result<Response, Error>> = Vec::new();

    for item in items {
        let article_title: String = match &item.title {
            Some(title_txt) => title_txt.clone(),
            None => "(NO TITLE)".to_string(),
        };
        let article_url: String = match item.link() {
            Some(url) => url.to_string(),
            None => "(NO URL)".to_string(),
        };

        let push_title: String = format!("NEW ARTICLE: {}", article_title);

        debug!("Sending a POST reqeust to ntfy for {} {}.", push_title, article_url);
        let result: Result<Response, Error> = client
            .post(format!("https://ntfy.sh/{ntfy_topic}"))
            .header("Title", push_title)
            .header("Message", article_url)
            .send();

        responses.push(result);
    }

    responses
}

/// **Purpose**:    Sends a POST to ntfy to make pushes for encountered errors
/// **Parameters**: A &[String] of encountered errors
/// **Ok Return**:  A reqwest::blocking::Response of the response received from ntfy
/// **Err Return**: A reqwest::Error from an unsuccessful POST
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn send_failure_notification(error_messages: &[String]) -> Result<Response, Error> {
    trace!(
        "Inside send_failure_notification with  {} error_messages.",
        error_messages.len()
    );
    let ntfy_topic: String = source_env_var("NTFY_TOPIC");

    let client: Client = Client::new();

    let push_title: String = format!("ERRORS: {}", error_messages.len());
    let mut error_string: String = String::new();

    for error in error_messages.iter() {
        error_string = error_string + error + " ";
    }

    debug!("Sending a POST reqeust to ntfy for {} errors.", error_messages.len());
    let result: Result<Response, Error> = client
        .post(format!("https://ntfy.sh/{ntfy_topic}"))
        .header("Title", push_title)
        .header("Message", error_string)
        .send();

    result
}

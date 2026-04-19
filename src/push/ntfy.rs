use crate::env_setup::source_env_var;
use log::{debug, trace};
use reqwest::{Error, blocking::Client, blocking::Response};
use rss::Item;
use std::error;

/// **Purpose**:    Sends a POST to ntfy to make pushes with article title and link of new rss content
/// **Parameters**: A &Vec<rss:Item> of new rss content
/// **Ok Return**:  A Vec<reqwest::blocking::Response> of all responses received from ntfy
/// **Err Return**: A Vec<reqwest::Error> from any unsuccessful POSTs
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn send_new_item_notification(
    items: &Vec<Item>,
) -> Vec<Result<Response, Box<dyn error::Error>>> {
    trace!(
        "Inside send_new_item_notification with {} items.",
        items.len()
    );
    let ntfy_topic: String = source_env_var("NTFY_TOPIC");

    let client: Client = Client::new();
    let mut responses: Vec<Result<Response, Box<dyn error::Error>>> = Vec::new();

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

        debug!(
            "Sending a POST reqeust to ntfy for {} {}.",
            push_title, article_url
        );
        let result: Result<Response, Box<dyn error::Error>> = match client
            .post(format!("https://ntfy.sh/{ntfy_topic}"))
            .header("Title", push_title)
            .body(article_url)
            .send()
        {
            Ok(ok) => Ok(ok),
            Err(err) => Err(Box::new(err)),
        };

        responses.push(result);
    }

    responses
}

/// **Purpose**:    Sends a POST to ntfy to make pushes for encountered errors
/// **Parameters**: A &[String] of encountered errors
/// **Ok Return**:  A reqwest::blocking::Response of the response received from ntfy
/// **Err Return**: A Box<dyn error::Error> from an unsuccessful POST
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn send_failure_notification(
    error_messages: &[String],
) -> Result<Response, Box<dyn error::Error>> {
    trace!(
        "Inside send_failure_notification with {} error_messages.",
        error_messages.len()
    );
    let ntfy_topic: String = source_env_var("NTFY_TOPIC");

    let client: Client = Client::new();

    let push_title: String = format!("ERRORS: {}", error_messages.len());
    let error_string: String = error_messages.join("\n");

    debug!(
        "Sending a POST reqeust to ntfy for {} errors.",
        error_messages.len()
    );
    let result: Result<Response, Error> = client
        .post(format!("https://ntfy.sh/{ntfy_topic}"))
        .header("Title", push_title)
        .body(error_string)
        .send();

    if let Err(result_err) = result {
        return Err(Box::new(result_err));
    }

    Ok(result.unwrap())
}

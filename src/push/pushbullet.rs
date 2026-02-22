use crate::env_setup::source_env_var;
use log::{debug, trace};
use reqwest::{Error, blocking::Client, blocking::Response};
use rss::Item;
use serde_json::{Value, json};

/// **Purpose**:    Sends a POST to pushbullet to make pushes with article title and link of new rss content
/// **Parameters**: A &Vec<rss:Item> of new rss content
/// **Ok Return**:  A Vec<reqwest::blocking::Response> of all responses received from pushbullet
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
    let pb_api_key: String = source_env_var("PB_API_KEY");

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
        let push_body: Value = json!({"body": article_url, "title": push_title, "type": "note"});

        debug!("Sending a POST reqeust to pushbullet with {}.", push_body);
        let result: Result<Response, Error> = client
            .post("https://api.pushbullet.com/v2/pushes")
            .header("Access-Token", pb_api_key.clone())
            .header("Content-Type", "application/json")
            .json(&push_body)
            .send();

        responses.push(result);
    }

    responses
}

/// **Purpose**:    Sends a POST to pushbullet to make pushes for encountered errors
/// **Parameters**: A &Vec<String> of encountered errors
/// **Ok Return**:  A Vec<reqwest::blocking::Response> of all responses received from pushbullet
/// **Err Return**: A reqwest::Error from an unsuccessful POST
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn send_failure_notification(error_messages: &Vec<String>) -> Result<Response, Error> {
    trace!(
        "Inside send_failure_notification with  {} error_messages.",
        error_messages.len()
    );
    let pb_api_key: String = source_env_var("PB_API_KEY");

    let client: Client = Client::new();

    let push_title: String = format!("ERRORS: {}", error_messages.len());
    let push_body: Value = json!({"body": error_messages, "title": push_title, "type": "note"});

    debug!("Sending a POST reqeust to pushbullet with {}.", push_body);
    let result: Result<Response, Error> = client
        .post("https://api.pushbullet.com/v2/pushes")
        .header("Access-Token", pb_api_key.clone())
        .header("Content-Type", "application/json")
        .json(&push_body)
        .send();

    result
}

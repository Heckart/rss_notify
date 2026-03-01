#![warn(clippy::all)]
#![deny(warnings)]
use bytes::Bytes;
use log::{debug, info, trace, warn};
use reqwest::StatusCode;
use reqwest::blocking::Response;
use rss::Item;
use rss_notify::env_setup::get_feed_list;
use rss_notify::fetch::fetch_feed_as_bytes;
use rss_notify::parse::get_new_rss_items;
use rss_notify::push::{send_failure_notification, send_new_item_notification};
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

/*
1. Download feed
2. Deserialize feed (using rss lib)
3. Grab new feed items
4. Alert on new entry
*/

/// **Purpose**:    Main driver for the rss_notify program
/// **Parameters**: None
/// **Returns**:    Nothing
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Add support for tracking website changes in addition to rss feed changes,
///                 maybe transition program from blocking to async, use If-Modified-Since to avoid
///                 downloading whole feed if unnecessary, use a sqlite db instead of txt files
fn main() {
    env_logger::init();
    trace!("Starting up!");

    // any recoverable errors are added to this vec. We will keep trying to send a push containing
    // all of the previously encountered errors
    let mut errors: Vec<String> = Vec::new();

    let feed_urls: Vec<String> = get_feed_list("RSS_NOTIFY_FEED_LIST");
    debug!("Sourced feed list of {} feeds.", feed_urls.len());

    // this program runs infinitely, set it and forget it
    loop {
        trace!("At the top of the main loop.");
        for url in feed_urls.iter() {
            // get the feed contents from the url
            let feed_bytes: Bytes = match fetch_feed_as_bytes(url.to_string()) {
                Ok(bytes) => {
                    debug!("Sourced feed bytes for {}.", url);
                    bytes
                }
                Err(err) => {
                    let err_msg: String = construct_full_error(&err);
                    warn!("fetch_feed_as_bytes: {}", err_msg);
                    try_send_failure_notification(&mut errors, Some(err_msg));
                    continue;
                }
            };

            // find any new items from the feed
            debug!("Looking for new items in {}.", url);
            let feed_items: Vec<Item> = match get_new_rss_items(feed_bytes) {
                Ok(items) => {
                    debug!("Grabbed feed items from {}.", url);
                    items
                }
                Err(err) => {
                    let err_msg: String = construct_full_error(&err);
                    warn!("get_new_rss_items: {}", err_msg);
                    try_send_failure_notification(&mut errors, Some(err_msg));
                    continue;
                }
            };

            //parse::print_serialized_rss(feed_items.clone());

            // if new items exist, send a push for them each
            if !feed_items.is_empty() {
                info!(
                    "{} new feed items exist from {}, so sending pushes.",
                    feed_items.len(),
                    url
                );

                let push_results: Vec<Result<Response, reqwest::Error>> =
                    send_new_item_notification(&feed_items);

                for response in push_results {
                    match response {
                        Ok(ok) => {
                            let status: StatusCode = ok.status();
                            let body: String = ok.text().unwrap();

                            if status != StatusCode::OK {
                                warn!("Ntfy gave non-OK response of {} for {}.", status, body);
                                errors.push(format!("The push {body} responded with {status}"));
                            } else {
                                debug!(
                                    "Ntfy responsed with\nStatus: {}\nBody:\n{}\n.",
                                    status, body
                                );
                            }
                        }
                        Err(err) => {
                            let err_msg: String = construct_full_error(&err);
                            warn!(
                                "send_new_item_notification: Initial response had errors: {}.",
                                err_msg
                            );
                            errors.push(err_msg.to_string());
                            debug!("Total errors are {}.", errors.len());
                        }
                    }
                }
            } else {
                info!("No new items found for {} since last check.", url);
            }

            if !errors.is_empty() {
                // perhaps there was a connection issue on our end, so lets wait a minute and see
                // if it clears itself up before we try to make another push
                info!(
                    "Errors are present, so sleeping for 60 seconds then trying to alert about them."
                );
                sleep(Duration::from_mins(1));
                try_send_failure_notification(&mut errors, None);
            }
        }
        // be nice an wait 5 minutes between updating feeds
        debug!("Sleeping for 5 mintes before looping again.");
        sleep(Duration::from_mins(5));
    }
}

/// **Purpose**:    Walks down the whole chain of error sources, adding each source to a String
/// **Parameters**: A &str with the erroring function's name, A &dyn Error with the function's error
/// **Returns**:    A string containing the whole chain of error sources from the provided &dyn Error
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn construct_full_error(err: &dyn Error) -> String {
    let mut err_message: String = format!("Failed to fetch feed bytes due to error: {err}");
    let mut current: &dyn Error = &err;
    // not using write macro here so theres no unwrap or extra error handling
    while let Some(source) = current.source() {
        err_message += "\nCaused by: ";
        err_message.push_str(&source.to_string());
        current = source;
    }

    err_message
}

/// **Purpose**:    Attempts to send a push containing error information (used for all errors that have not been
///                 sent in a push yet)
/// **Parameters**: A &mut Vec<String> of encountered errors, a Option<String> of a new error to
///                 add to the error vector
/// **Returns**:    Nothing
/// **Panics**:     No
/// **Modifies**:   Appends an error to the errors vector is one is supplied, Clears the errors vector if a successful push occurs
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn try_send_failure_notification(errors: &mut Vec<String>, new_error: Option<String>) {
    if let Some(err) = new_error {
        errors.push(err);
    }

    trace!(
        "Inside try_send_failure_notification error count {}.",
        errors.len()
    );
    debug!("Total errors are {}.", errors.len());

    match send_failure_notification(errors) {
        Ok(ok) => {
            debug!(
                "Ntfy responsed with\nStatus: {}\nBody:\n{}\n.",
                ok.status(),
                ok.text().unwrap()
            );
            info!("Able to send error notification, so clearing error vector.");
            errors.clear();
        }
        Err(err) => {
            let err_msg: String = construct_full_error(&err);
            warn!("Attempt to send errors had errors {}.", err_msg);
            errors.push(err_msg);
            debug!("Total errors are {}.", errors.len());
        }
    }
}

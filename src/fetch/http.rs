use bytes::Bytes;
use log::trace;
use reqwest::blocking::get;
use reqwest::Error;

/// **Purpose**:    Grab the bytes of rss feed content
/// **Parameters**: A String containing an rss feed URL
/// **Ok Return**:  A Bytes object of rss content
/// **Err Return**: A reqwest::Error from a GET request
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Switch to using etags/last modified date to see if we need to pull the whole
///                 feed instead of actually pulling the whole thing every time
pub fn fetch_feed_as_bytes(feed_url: String) -> Result<Bytes, Error> {
    trace!("Inside fetch_feed_as_bytes with feed_url of {}.", feed_url);
    let feed_bytes: Bytes = get(feed_url)?.bytes()?;

    Ok(feed_bytes)
}

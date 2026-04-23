use crate::database::{DBEntry, feed_is_in_db, insert_feed_to_db};
use crate::parse::stringify_feed_bytes;
use bytes::Bytes;
use log::{debug, error, trace};
use reqwest::blocking::get;
use rusqlite::Connection;
use std::error;

//struct ResponseHeaders {
//    pub etag: String,
//    pub last_modified: String,
//}

/// **Purpose**:    Grab the bytes of rss feed content if new content is available
/// **Parameters**: A &rusqlite::Connection for the db connection, a &String containing an rss feed URL
/// **Ok Return**:  An Option<Bytes> object of rss content if a new GET request was made
/// **Err Return**: A Box<dyn error::Error> from a GET request or DB query
/// **Panics**:     No
/// **Modifies**:   Creates a new DB row if the feed didn't already have an entry
/// **Tests**:      Not implemented yet
/// **Status**:     Switch to using etags/last modified date to see if we need to pull the whole
///                 feed instead of actually pulling the whole thing every time
pub fn fetch_feed_as_bytes(
    conn: &Connection,
    feed_url: &String,
) -> Result<Option<Bytes>, Box<dyn error::Error>> {
    trace!("Inside fetch_feed_as_bytes with feed_url of {}.", feed_url);

    let mut returned_feed_bytes: Option<Bytes> = None;

    match feed_is_in_db(conn, feed_url) {
        Ok(feed_present) => {
            if feed_present {
                // get the DBEntry for the feed.
                // TODO: Request the feed-bytes making use of etag or last-modified headers if they
                // exist
                let feed_bytes: Bytes = match make_get_request(feed_url) {
                    Ok(bytes) => {
                        trace!("Received feed bytes from {}.", feed_url);
                        bytes
                    }
                    Err(err) => {
                        return Err(err);
                    }
                };
                returned_feed_bytes = Some(feed_bytes)
            } else {
                // its the first time we've seen the feed, so pull its headers+bytes, parse into db and
                // return None, so we know to continue in the main function
                let feed_bytes: Bytes = match make_get_request(feed_url) {
                    Ok(bytes) => {
                        trace!("Received feed bytes from {}.", feed_url);
                        bytes
                    }
                    Err(err) => {
                        return Err(err);
                    }
                };

                let new_row: DBEntry = DBEntry {
                    feed_name: feed_url.to_string(),
                    history: match stringify_feed_bytes(feed_bytes) {
                        Ok(feed_history) => {
                            trace!("Received rss feed history string.");
                            feed_history
                        }
                        Err(err) => {
                            error!("Could not update DB feed history.");
                            return Err(err);
                        }
                    },
                    last_modified: None,
                    etag: None,
                };

                match insert_feed_to_db(conn, new_row) {
                    Ok(_) => {
                        debug!("Inserted new row to DB for first feed encounter.");
                    }
                    Err(err) => {
                        error!("Could not update DB row for {}.", feed_url);
                        return Err(Box::new(err));
                    }
                }
            }
        }
        Err(err) => return Err(Box::new(err)),
    };

    Ok(returned_feed_bytes)
}

/// **Purpose**:    Helper function to perform a GET request on a rss feed url
/// **Parameters**: A &String containing an rss feed url
/// **Ok Return**:  A Bytes object of the rss feed contents
/// **Err Return**: A Box<dyn error::Error> from a GET request
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn make_get_request(feed_url: &String) -> Result<Bytes, Box<dyn error::Error>> {
    match get(feed_url) {
        Ok(response) => match response.bytes() {
            Ok(bytes) => {
                trace!("GET request for {} successful.", feed_url);
                Ok(bytes)
            }
            Err(err) => {
                error!("Conversion of Response to bytes for {} failed.", feed_url);
                Err(Box::new(err))
            }
        },
        Err(err) => {
            error!("Get request for {} failed.", feed_url);
            Err(Box::new(err))
        }
    }
}

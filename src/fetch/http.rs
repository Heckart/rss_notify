use crate::database::{DBEntry, feed_is_in_db, get_feed_from_db, insert_feed_to_db};
use crate::parse::stringify_feed_bytes;
use bytes::Bytes;
use log::{debug, error, trace, warn};
use reqwest::StatusCode;
use reqwest::blocking::{Client, RequestBuilder, Response, get};
use reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use rusqlite::Connection;
use std::error;

/// Holds the three members of a reqwest resonse that we care about
struct ResponseDetails {
    pub bytes: Option<Bytes>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub first_time_feed: bool,
}

/// **Purpose**:    Grab the bytes of rss feed content if new content is available
/// **Parameters**: A &rusqlite::Connection for the db connection, a &String containing an rss feed URL
/// **Ok Return**:  An Option<Bytes> object of rss content if a new GET request was made
/// **Err Return**: A Box<dyn error::Error> from a GET request or DB query
/// **Panics**:     No
/// **Modifies**:   Creates a new DB row if the feed didn't already have an entry, updates the DB
///                 row headers if there are changes in the feed
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn fetch_feed_as_bytes(
    conn: &Connection,
    feed_url: &String,
) -> Result<Option<Bytes>, Box<dyn error::Error>> {
    trace!("Inside fetch_feed_as_bytes with feed_url of {feed_url}.");

    let feed_response: ResponseDetails;

    match feed_is_in_db(conn, feed_url) {
        Ok(feed_present) => {
            if feed_present {
                // its not the first time we've seen the feed, so make a conditional request based
                // upon the existing header contents if there are any
                feed_response = match get_existing_feed(conn, feed_url) {
                    Ok(url_response) => {
                        trace!("GET routine for existing feed {feed_url} successful.");
                        url_response
                    }
                    Err(err) => {
                        error!("GET routine for existing feed {feed_url} failed.");
                        return Err(err);
                    }
                };
            } else {
                // its the first time we've seen the feed, so pull its headers+bytes, parse into db and
                // return None, so we know to continue in the main function
                feed_response = match make_get_request(feed_url, None) {
                    Ok(url_response) => {
                        trace!("GET routine for new feed {feed_url} successful.");
                        url_response
                    }
                    Err(err) => {
                        error!("GET routine for new  feed {feed_url} failed.");
                        return Err(err);
                    }
                };
            }
        }
        Err(err) => return Err(Box::new(err)),
    }

    let mut returned_feed_bytes: Option<Bytes> = None;

    if let Some(new_feed_bytes) = feed_response.bytes.clone() {
        returned_feed_bytes = Some(new_feed_bytes);

        // The only time this function should update the feed history is if the feed doesn't already
        // exist in the data base
        let bytes_to_stringify: Option<Bytes> = match feed_response.first_time_feed {
            false => None,
            true => feed_response.bytes,
        };
        match update_db_with_new_feed_info(
            conn,
            feed_url,
            bytes_to_stringify,
            feed_response.etag,
            feed_response.last_modified,
        ) {
            Ok(_) => {
                trace!("Successful update to DB for {feed_url}.");
            }
            Err(err) => {
                error!("Could not update DB for {feed_url}.");
                return Err(err);
            }
        }
    }

    Ok(returned_feed_bytes)
}

/// **Purpose**:    Helper function to perform a GET request on a rss feed url and return useful
///                 information
/// **Parameters**: A &String containing an rss feed url, an
///                 Option<RequestBuilder> if the search will also be based
///                 on etag/last-modified headers
/// **Ok Return**:  A ResponseDeatils object of the rss feed information
/// **Err Return**: A Box<dyn error::Error> from a GET request
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn make_get_request(
    feed_url: &String,
    get_client: Option<RequestBuilder>,
) -> Result<ResponseDetails, Box<dyn error::Error>> {
    trace!("Inside make_get_request.");

    let request_result: Result<Response, reqwest::Error>;

// tracking purposes in fetch_feed_as_bytes(), makes that function simpler 
    let first_time_seeing_feed: bool = if let Some(get_request) = get_client {
        request_result = RequestBuilder::send(get_request);
        false
    } else {
        request_result = get(feed_url);
        true
    };

    let response: Response = match request_result {
        Ok(url_response) => {
            trace!("GET request for {feed_url} successful.");
            url_response
        }
        Err(err) => {
            error!("GET request for {feed_url} failed.");
            return Err(Box::new(err));
        }
    };

    let response_etag: Option<String>;
    let response_last_modified: Option<String>;
    let response_bytes: Option<Bytes>;
    match response.status() {
        StatusCode::NOT_MODIFIED => {
            debug!(
                "GET request for {feed_url} had matching headers! Assuming no change to feed contents."
            );
            //TODO: Can probably still update headers?
            response_etag = None;
            response_last_modified = None;
            response_bytes = None;
        }
        StatusCode::OK => {
            // full request was made
            debug!("Full GET request made for {feed_url}! Assuming new feed contents.");
            //TODO: These two are supressing errors with ok(), so at some point make the errors explicit
            response_etag = response
                .headers()
                .get(ETAG)
                .and_then(|header| header.to_str().ok())
                .map(str::to_owned);

            response_last_modified = response
                .headers()
                .get(LAST_MODIFIED)
                .and_then(|header| header.to_str().ok())
                .map(str::to_owned);

            response_bytes = match response.bytes() {
                Ok(bytes) => {
                    trace!("Extracted bytes from response.");
                    Some(bytes)
                }
                Err(err) => {
                    error!("Could not extract bytes from response.");
                    return Err(Box::new(err));
                }
            };
        }
        other_rc => {
            warn!(
                "GET request for {feed_url} had unexpected status code {other_rc}! Not sure what happened, so doing nothing."
            );
            // Don't update the headers here. We may have missed content and want to grab it next time.
            response_etag = None;
            response_last_modified = None;
            response_bytes = None;
        }
    }

    Ok(ResponseDetails {
        bytes: response_bytes,
        etag: response_etag,
        last_modified: response_last_modified,
        first_time_feed: first_time_seeing_feed,
    })
}

/// **Purpose**:    Based on current feed headers and DB headers, pull down the feed bytes only if
///                 necessary
/// **Parameters**: A &String containing an rss feed url
/// **Ok Return**:  A ResponseDetails object from the feed
/// **Err Return**: A Box<dyn error::Error> from failure to read the DB
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn get_existing_feed(
    conn: &Connection,
    feed_url: &String,
) -> Result<ResponseDetails, Box<dyn error::Error>> {
    trace!("Inside get_existing_feed().");
    let existing_db_entry: DBEntry = match get_feed_from_db(conn, feed_url) {
        Ok(db_row) => {
            trace!("Successfully sourced existing DB entry for {feed_url}.");
            db_row
        }
        Err(err) => {
            error!("Could not successfully source existing DB entry for {feed_url}.");
            return Err(Box::new(err));
        }
    };

    let mut get_client: RequestBuilder = Client::new().get(feed_url);

    // maybe change this to do both etag and last_modified if both exist?
    if let Some(etag) = &existing_db_entry.etag {
        get_client = get_client.header(IF_NONE_MATCH, etag);
    } else if let Some(last_modified) = &existing_db_entry.last_modified {
        get_client = get_client.header(IF_MODIFIED_SINCE, last_modified);
    } else {
        // there aren't any saved headers, so we need to pull the full bytes not considering headers
        return make_get_request(feed_url, None);
    }

    make_get_request(feed_url, Some(get_client))
}

/// **Purpose**:    Update or create DB entry for a feeds headers + contents if feed is new
/// **Parameters**: A &rusqlite::Connection for the db connection, a &String with feed name, a
///                 Bytes object with new feed bytes, two Option<String>s for the etag and
///                 last_modified headers
/// **Ok Return**:  A usize with row change count from the DB
/// **Err Return**: A Box<dyn error::Error> from failure stringify bytes or to read the DB
/// **Panics**:     No
/// **Modifies**:   Creates or updates a row in the DB
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn update_db_with_new_feed_info(
    conn: &Connection,
    feed_url: &String,
    feed_bytes: Option<Bytes>,
    feed_etag: Option<String>,
    feed_last_modified: Option<String>,
) -> Result<usize, Box<dyn error::Error>> {
    trace!("Inside update_db_with_new_feed_info().");

    let mut new_row: DBEntry = DBEntry {
        feed_name: feed_url.clone(),
        history: None,
        last_modified: feed_last_modified,
        etag: feed_etag,
    };

    if let Some(new_bytes) = feed_bytes {
        new_row.history = match stringify_feed_bytes(&new_bytes) {
            Ok(feed_history) => {
                trace!("Received rss feed history string.");
                Some(feed_history)
            }
            Err(err) => {
                error!("Could not stringify feed_bytes from ().");
                return Err(err);
            }
        }
    }

    match insert_feed_to_db(conn, &new_row) {
        Ok(rc) => {
            debug!("DB responded with {rc} after insert for {feed_url}.");
            Ok(rc)
        }
        Err(err) => {
            error!("Could not update DB row for {feed_url}.");
            Err(Box::new(err))
        }
    }
}

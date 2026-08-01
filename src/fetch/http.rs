use crate::database::{
    DBEntry, DBHeaders, feed_is_in_db, get_feed_from_db, insert_feed_to_db, update_feed_headers,
};
use crate::parse::stringify_feed_bytes;
use bytes::Bytes;
use log::{debug, error, trace, warn};
use reqwest::StatusCode;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{
    ETAG, HeaderName, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED, ToStrError,
};
use rusqlite::Connection;
use std::error;

/// Holds the three members of a reqwest resonse that we care about
/// bytes is optional in case no result is returned. If it didn't have the possibility of being
/// optional, we could just use FeedBytesAndHeaders instead
struct ResponseDetails {
    pub bytes: Option<Bytes>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub response_type: StatusCode,
}

/// Information to be passed from the fetch library to the parse library. Contains the feed bytes as
/// well as the relevant headers if they exist
pub struct FeedBytesAndHeaders {
    pub bytes: Bytes,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// **Purpose**:    Grab the bytes of rss feed content if new content is available
/// **Parameters**: A &rusqlite::Connection for the db connection, a &String containing an rss feed URL,
/// **Ok Return**:  An Option<FeedBytesAndHeaders> object of rss content and headers if a new GET
///                 request was made and new content is expected
/// **Err Return**: A Box<dyn error::Error> from a GET request or DB query
/// **Panics**:     No
/// **Modifies**:   Creates a new DB row if the feed didn't already have an entry
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn fetch_feed_as_bytes(
    conn: &Connection,
    feed_url: &String,
) -> Result<Option<FeedBytesAndHeaders>, Box<dyn error::Error>> {
    #![allow(clippy::too_many_lines)]
    trace!("Inside fetch_feed_as_bytes with feed_url of {feed_url}.");

    let feed_response: ResponseDetails;
    let first_time_feed: bool;
    // we need to initialize this here to avoid a possible unintialized referece. Maybe this could
    // be converted to a non-optional with junk defaulted values?
    let mut existing_db_entry: Option<DBEntry> = None;

    match feed_is_in_db(conn, feed_url) {
        Ok(feed_present) => {
            if feed_present {
                first_time_feed = false;
                // its not the first time we've seen the feed, so make a conditional request based
                // upon the existing header contents if there are any
                existing_db_entry = match get_feed_from_db(conn, feed_url) {
                    Ok(row) => {
                        trace!("Received row from DB.");
                        Some(row)
                    }
                    Err(err) => {
                        error!("Failed to receive row from DB.");
                        return Err(Box::new(err));
                    }
                };
                feed_response = match get_existing_feed(feed_url, existing_db_entry.clone()) {
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
                first_time_feed = true;
                // its the first time we've seen the feed, so pull its headers+bytes, parse into db and
                // return None, so we know to continue in the main function
                feed_response = match make_get_request(feed_url, Client::new().get(feed_url)) {
                    Ok(url_response) => {
                        trace!("GET routine for new feed {feed_url} successful.");
                        url_response
                    }
                    Err(err) => {
                        error!("GET routine for new feed {feed_url} failed.");
                        return Err(err);
                    }
                };
            }
        }
        Err(err) => return Err(Box::new(err)),
    }

    let returned_feed_elements: FeedBytesAndHeaders;

    // The bytes condition is what determines whether or not there is genuinely new content
    if (feed_response.response_type == StatusCode::OK)
        && let Some(new_feed_bytes) = feed_response.bytes
    {
        returned_feed_elements = FeedBytesAndHeaders {
            bytes: new_feed_bytes,
            etag: feed_response.etag,
            last_modified: feed_response.last_modified,
        };

        let hacky_debugging_fake_struct = DBEntry {
            feed_name: String::new(),
            history: String::new(),
            last_modified: None,
            etag: None,
        };
        debug!(
            "Pulled {feed_url} and received OK response and bytes.\nExisting Last-Modified header: {}. New Last-Modified: {}\nExisting ETag: {}. New ETag: {}.",
            existing_db_entry
                .as_ref()
                .unwrap_or(&hacky_debugging_fake_struct)
                .last_modified
                .as_deref()
                .unwrap_or("<None>"),
            returned_feed_elements
                .last_modified
                .as_deref()
                .unwrap_or("<None>"),
            existing_db_entry
                .as_ref()
                .unwrap_or(&hacky_debugging_fake_struct)
                .etag
                .as_deref()
                .unwrap_or("<None>"),
            returned_feed_elements.etag.as_deref().unwrap_or("<None>"),
        );
        // only calling this function if we know its the first time the feed has been seen
        if first_time_feed {
            match update_db_with_new_feed_info(conn, feed_url, &returned_feed_elements) {
                Ok(_) => {
                    trace!("Successful update to DB for {feed_url}.");
                    return Ok(None); // first time feed means return nothing back to main
                }
                Err(err) => {
                    error!("Could not update DB for {feed_url}.");
                    return Err(err);
                }
            }
        }
        // Not a new feed, so headers will be updated in parse step.
        Ok(Some(returned_feed_elements))
    } else if feed_response.response_type == StatusCode::NOT_MODIFIED {
        if let Some(current_db_entry) = existing_db_entry
            && (feed_response.etag != current_db_entry.etag
                || feed_response.last_modified != current_db_entry.last_modified)
        {
            debug!(
                "Pulled {feed_url} and received NOT_MODIFIED response with different headers\nExisting Last-Modified: {}. New Last-Modified: {}\nExisting ETag: {}. New ETag: {}.",
                current_db_entry
                    .last_modified
                    .as_deref()
                    .unwrap_or("<None>"),
                feed_response.last_modified.as_deref().unwrap_or("<None>"),
                current_db_entry.etag.as_deref().unwrap_or("<None>"),
                feed_response.etag.as_deref().unwrap_or("<None>")
            );

            match update_feed_headers(
                conn,
                &DBHeaders {
                    feed_name: current_db_entry.feed_name,
                    etag: feed_response.etag.clone(),
                    last_modified: feed_response.last_modified.clone(),
                },
            ) {
                Ok(_) => {
                    trace!("Headers updated successfully.");
                    Ok(None)
                }
                Err(err) => {
                    error!("Headers could not be updated due to {err}.");
                    Err(Box::new(err))
                }
            }
        } else {
            trace!(
                "Pulled {feed_url} and received NOT_MODIFED response with matching headers, so no update needed. Last-Modified: {}. ETag: {}.",
                feed_response.last_modified.as_deref().unwrap_or("<None>"),
                feed_response.etag.as_deref().unwrap_or("<None>")
            );
            Ok(None)
        }
    } else {
        let hacky_debugging_fake_struct = DBEntry {
            feed_name: String::new(),
            history: String::new(),
            last_modified: None,
            etag: None,
        };

        warn!(
            "Pulled {feed_url} and received no bytes back, not updating headers.\nExisting Last-Modified header: {}. New Last-Modified: {}\nExisting ETag: {}. New ETag: {}.",
            existing_db_entry
                .as_ref()
                .unwrap_or(&hacky_debugging_fake_struct)
                .last_modified
                .as_deref()
                .unwrap_or("<None>"),
            feed_response.last_modified.as_deref().unwrap_or("<None>"),
            existing_db_entry
                .as_ref()
                .unwrap_or(&hacky_debugging_fake_struct)
                .etag
                .as_deref()
                .unwrap_or("<None>"),
            feed_response.etag.as_deref().unwrap_or("<None>"),
        );

        Ok(None) // no bytes were returned from earlier, so we know there is no new content to parse
    }
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
    get_client: RequestBuilder,
) -> Result<ResponseDetails, Box<dyn error::Error>> {
    trace!("Inside make_get_request.");

    let response: Response = match RequestBuilder::send(get_client) {
        Ok(url_response) => {
            trace!("GET request for {feed_url} successful.");
            url_response
        }
        Err(err) => {
            error!("GET request for {feed_url} failed.");
            return Err(Box::new(err));
        }
    };

    let bytes: Option<Bytes>;
    let etag: Option<String>;
    let last_modified: Option<String>;
    let response_type: StatusCode;

    match response.status() {
        StatusCode::NOT_MODIFIED => {
            debug!(
                "GET request for {feed_url} had matching headers! Assuming no change to feed contents."
            );
            // In theory, the server will choose to match on only one of the headers if both are
            // sent, meaning one could change and we still get NOT_MODIFIED response. We will pass ahead
            // those values now just in case, though this situation admittedly seems like a rare
            // occurence
            etag = match extract_header_as_string(&response, &ETAG) {
                Ok(header_str) => {
                    trace!("New (presumably matching) ETag successfully extraced.");
                    header_str
                }
                Err(err) => {
                    error!("ETag extraction for {feed_url} failed.");
                    return Err(Box::new(err));
                }
            };
            last_modified = match extract_header_as_string(&response, &LAST_MODIFIED) {
                Ok(header_str) => {
                    debug!("New (presumably matching) Last-Modified successfully extracted.");
                    header_str
                }
                Err(err) => {
                    error!("Last-Modified extraction for {feed_url} failed.");
                    return Err(Box::new(err));
                }
            };
            bytes = None;
            response_type = StatusCode::NOT_MODIFIED;
        }
        StatusCode::OK => {
            // full request was made
            debug!("Full GET request made for {feed_url}! Assuming new feed contents.");

            etag = match extract_header_as_string(&response, &ETAG) {
                Ok(header_str) => {
                    debug!("New ETag header successfully extraced.");
                    header_str
                }
                Err(err) => {
                    error!("ETag extraction for {feed_url} failed.");
                    return Err(Box::new(err));
                }
            };
            last_modified = match extract_header_as_string(&response, &LAST_MODIFIED) {
                Ok(header_str) => {
                    debug!("New Last-Modified header successfully extracted.");
                    header_str
                }
                Err(err) => {
                    error!("Last-Modified extraction for {feed_url} failed.");
                    return Err(Box::new(err));
                }
            };
            bytes = match response.bytes() {
                Ok(resp_bytes) => {
                    trace!("Extracted bytes from response.");
                    Some(resp_bytes)
                }
                Err(err) => {
                    error!("Bytes extraction for {feed_url} failed.");
                    return Err(Box::new(err));
                }
            };
            response_type = StatusCode::OK;
        }
        other_rc => {
            // most likely this ends up being a 412 based on the standard
            warn!(
                "GET request for {feed_url} had unexpected status code {other_rc}! Not sure what happened, so doing nothing."
            );
            // Don't update the headers here. We may have missed content and want to grab it next time.
            etag = None;
            last_modified = None;
            bytes = None;
            response_type = other_rc;
        }
    }

    Ok(ResponseDetails {
        bytes,
        etag,
        last_modified,
        response_type,
    })
}

/// **Purpose**:    Helper function to extract a header from a response if it exists and stringify it
/// **Parameters**: A Response with headers to extract, A HeaderName of the header type to extract
/// **Ok Return**:  An Option<String> with the header contents if it existed
/// **Err Return**: A ToStrError from failure to stringify
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn extract_header_as_string(
    response: &Response,
    header_type: &HeaderName,
) -> Result<Option<String>, ToStrError> {
    let binding = header_type.clone();
    let header_name_str: &str = binding.as_str();
    response.headers().get(header_type).map_or_else(
        || {
            trace!("Feed does not have {header_name_str}.");
            Ok(None)
        },
        |header_val| match header_val.to_str() {
            Ok(header_str) => {
                trace!("{header_name_str} stringified as {header_str}.");
                Ok(Some(header_str.to_owned()))
            }
            Err(err) => {
                error!("{header_name_str} exists, but cannot be stringified. {err}");
                Err(err)
            }
        },
    )
}

/// **Purpose**:    Based on current feed headers and DB headers, pull down the feed bytes only if
///                 necessary
/// **Parameters**: A &String containing an rss feed url, a Option<DBEntry> representing the current
///                 state of the DB
/// **Ok Return**:  A ResponseDetails object from the feed
/// **Err Return**: A Box<dyn error::Error> from failure to read the DB
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn get_existing_feed(
    feed_url: &String,
    existing_db_entry: Option<DBEntry>,
) -> Result<ResponseDetails, Box<dyn error::Error>> {
    trace!("Inside get_existing_feed().");

    let mut get_client: RequestBuilder = Client::new().get(feed_url);

    // have to do some trickery to satisfy requirements in the main function from this library
    // In practice, every time this function gets called, the DB will exist.
    if let Some(current_db_entry) = existing_db_entry {
        // you can send multiple conditional headers
        // https://datatracker.ietf.org/doc/html/rfc7232#section-6
        if let Some(etag) = current_db_entry.etag {
            get_client = get_client.header(IF_NONE_MATCH, etag);
        }
        if let Some(last_modified) = current_db_entry.last_modified {
            get_client = get_client.header(IF_MODIFIED_SINCE, last_modified);
        }
    }

    make_get_request(feed_url, get_client)
}

/// **Purpose**:    Update or create DB entry for a feeds headers + contents if feed is new
/// **Parameters**: A &rusqlite::Connection for the db connection, a &FeedBytesAndHeaders with the
///                 bytes amd headers from recent feed pull
/// **Ok Return**:  A usize with row change count from the DB
/// **Err Return**: A Box<dyn error::Error> from failure stringify bytes or to read the DB
/// **Panics**:     No
/// **Modifies**:   Creates a row in the DB
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn update_db_with_new_feed_info(
    conn: &Connection,
    feed_url: &String,
    feed_bytes_headers: &FeedBytesAndHeaders,
) -> Result<usize, Box<dyn error::Error>> {
    trace!("Inside update_db_with_new_feed_info().");

    let new_row: DBEntry = DBEntry {
        feed_name: feed_url.clone(),
        history: match stringify_feed_bytes(&feed_bytes_headers.bytes) {
            Ok(feed_history) => {
                trace!("Received rss feed history string.");
                feed_history
            }
            Err(err) => {
                error!("Could not stringify feed_bytes from ().");
                return Err(err);
            }
        },
        last_modified: feed_bytes_headers.last_modified.clone(),
        etag: feed_bytes_headers.etag.clone(),
    };

    match insert_feed_to_db(conn, &new_row) {
        Ok(rc) => {
            trace!("DB responded with {rc} after insert for {feed_url}.");
            Ok(rc)
        }
        Err(err) => {
            error!("Could not update DB row for {feed_url}.");
            Err(Box::new(err))
        }
    }
}

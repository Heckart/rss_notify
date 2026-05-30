use crate::database::{DBEntry, get_feed_from_db, insert_feed_to_db};
use bytes::Bytes;
use log::{debug, error, trace, warn};
use rss::{Channel, Item, ItemBuilder};
use rusqlite::Connection;
use std::error;

/// **Purpose**:    Grab new items from an rss feed and maintains DB feed history
/// **Parameters**: A &rusqlite:Connection of the history db, a &String of the feed url, A Bytes object of rss content
/// **Ok Return**:  A Vec<rss::Item> of previously unseen rss content
/// **Err Return**: A Box<dyn error::Error> from failure to serialize feed, failure to get feed
///                 bytes, or failure to access the DB
/// **Panics**:     No
/// **Modifies**:   If new rss::Items are found, updates the DB history for the feed
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn get_new_rss_items(
    conn: &Connection,
    feed_url: &String,
    feed_bytes: &Bytes,
) -> Result<Vec<Item>, Box<dyn error::Error>> {
    #![allow(clippy::expect_used)] // temporary
    trace!("Inside get_new_rss_items.");

    let db_feed_items: Vec<Item> = match get_feed_from_db(conn, feed_url) {
        Ok(response) => match serde_json::from_str(
            response
                .history
                .expect("DB invariant violation: history must be NOT NULL.")
                .as_str(),
        ) {
            Ok(items) => {
                trace!("Successfully serialized {feed_url} rss items from DB.");
                items
            }
            Err(err) => {
                error!("Failed to turn DB feed history string into Vec<rss:Item>.");
                return Err(Box::new(err));
            }
        },
        Err(err) => {
            error!("Failed to get {feed_url} row from DB.");
            return Err(Box::new(err));
        }
    };

    let new_feed_channel: Channel = match Channel::read_from(&**feed_bytes) {
        Ok(channel) => {
            trace!("Successfully converted {feed_url} feed bytes into rss channel.");
            normalize_rss_items_in_channel(channel)
        }
        Err(err) => {
            error!("Failed to convert feed bytes to rss channel.");
            return Err(Box::new(err));
        }
    };

    let new_items: Vec<Item> = make_new_item_vector(&db_feed_items, &new_feed_channel);

    // if there are differences, update db row and return the new items
    if !new_items.is_empty() {
        let updated_row: DBEntry = DBEntry {
            feed_name: feed_url.clone(),
            history: match stringify_feed_bytes(feed_bytes) {
                Ok(feed_hist) => {
                    trace!("Successfully stringified feed bytes for insertion to DB");
                    Some(feed_hist)
                }
                Err(err) => {
                    error!("Could not stringify feed bytes for insertion to DB");
                    return Err(err);
                }
            },
            last_modified: None,
            etag: None,
        };

        // technically we don't have to return the error here, but it makes more sense to.
        // if we can't update here, we will find the changes from this iteration again next time
        // as it will still be comparing with the old DB data
        match insert_feed_to_db(conn, &updated_row) {
            Ok(_) => {
                trace!("Sucessfully updated {feed_url} DB row");
            }
            Err(err) => {
                error!("Could not update DB row for {feed_url}.");
                return Err(Box::new(err));
            }
        }
    }

    Ok(new_items)
}

/// **Purpose**:    Serialize a Bytes object of rss content into a String with only title and link
///                 taken from each rss item
/// **Parameters**: A Bytes object of rss content
/// **Ok Return**:  A String of rss content
/// **Err Return**: A Box<dyn error::Error) if bytes cannot be converted to rss channel or rss::Item
///                 vector can't be serialized
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn stringify_feed_bytes(feed_bytes: &Bytes) -> Result<String, Box<dyn error::Error>> {
    trace!("Inside serialize_feed_bytes");
    let rss_channel: Channel = match Channel::read_from(&**feed_bytes) {
        Ok(result) => {
            trace!("Successfully converted feed bytes to rss_channel.");
            normalize_rss_items_in_channel(result)
        }
        Err(err) => {
            error!("Couldn't convert bytes to rss Channel due to error: {err}.");
            return Err(Box::new(err));
        }
    };

    let serialized: String = match serde_json::to_string(&rss_channel.items().to_vec()) {
        Ok(json) => {
            debug!("Successfully stringified feed bytes as {json}.");
            json
        }
        Err(err) => {
            error!("Couldn't serialize item vector! {err}");
            return Err(Box::new(err));
        }
    };

    Ok(serialized)
}

/// **Purpose**:    Normalize the Items in an rss::Channel to only include title and link. All other
///                 fields become null
/// **Parameters**: A rss::Channel
/// **Return**:     A rss::Channel with its Items normalized
/// **Panics**:     No
/// **Modifies**:   The Item vector of the rss::Channel
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn normalize_rss_items_in_channel(channel: Channel) -> Channel {
    trace!("Inside normalize_rss_items_in_channel.");
    let mut normalized_channel: Channel = channel;
    let mut new_items: Vec<Item> = Vec::new();
    // we only serialize the url and article title to avoid false "new item" reports. The article
    // body could contain date-specific elements that change from pull to pull to pull
    for item in normalized_channel.items {
        let item_title: &str = item.title().map_or_else(
            || {
                warn!("Item had no title, inserting as 'N/A'.");
                "N/A"
            },
            |title| {
                trace!("Item had title {title}");
                title
            },
        );

        let item_link: &str = item.link().map_or_else(
            || {
                warn!("Item had no link, inserting as 'N/A'.");
                "N/A"
            },
            |link| {
                trace!("Item had link {link}");
                link
            },
        );

        new_items.push(
            ItemBuilder::default()
                .title(Some(item_title.to_owned()))
                .link(Some(item_link.to_owned()))
                .build(),
        );
    }

    normalized_channel.items = new_items;
    normalized_channel
}

/// **Purpose**:    Constructs a vector of previously unseen rss content
/// **Parameters**: A Vec<rss:Item> of previously seen rss content
///                 A rss::Channel of a all content on an rss feed
/// **Returns**:    A Vec<rss::Item> of rss content in the passed Channel that is not in the passed Vec
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn make_new_item_vector(old_items: &[Item], new_channel: &Channel) -> Vec<Item> {
    trace!(
        "Inside make_new_item_vector with {} old_items.",
        old_items.len()
    );

    let mut new_items: Vec<Item> = Vec::new();

    for item in new_channel.items() {
        if !old_items.contains(item) {
            new_items.push(item.clone());
        }
    }

    new_items
}

/// **Purpose**:    Pretty prints almost every possible rss Item field if they are populated
/// **Parameters**: A Vec<rss:Item> of rss content
/// **Returns**:    Nothing
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn print_serialized_rss(items: &[Item]) {
    #![allow(clippy::print_stdout)]
    trace!("Inside print_serialized_rss with  {} items.", items.len());
    if !items.is_empty() {
        for item in items {
            if let Some(thing) = item.title() {
                println!("TITLE IS: {thing}");
            }
            if let Some(thing) = item.link() {
                println!("LINK IS: {thing}");
            }
            if let Some(thing) = item.description() {
                println!("DESC IS: {thing}");
            }
            if let Some(thing) = item.author() {
                println!("AUTHOR IS: {thing}");
            }
            if !item.categories().is_empty() {
                for category in item.categories() {
                    println!("CATEGORY NAME IS: {}", category.name());
                    if let Some(thing) = category.domain() {
                        println!("CATEGORY DOMAIN IS: {thing}");
                    }
                }
            }
            if let Some(thing) = item.enclosure() {
                println!("ENCLOSURE URL IS: {}", thing.url());
                println!("ENCLOSURE LENGTH IS: {}", thing.length());
                println!("ENCLOSURE MIME TYPE IS: {}", thing.mime_type());
            }
            if let Some(thing) = item.guid() {
                println!("GUID VALUE IS: {}", thing.value());
                match thing.is_permalink() {
                    true => {
                        println!("GUID PERMALINK IS TRUE");
                    }
                    false => {
                        println!("GUID PERMALINK IS FALSE");
                    }
                }
            }
            if let Some(thing) = item.comments() {
                println!("COMMENTS IS: {thing}");
            }
            if let Some(thing) = item.pub_date() {
                println!("PUB DATE IS: {thing}");
            }
            if let Some(thing) = item.content() {
                println!("CONTENT IS: {thing}");
            }
        }
    }
}

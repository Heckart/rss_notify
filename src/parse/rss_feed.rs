use crate::database::{DBEntry, get_feed_from_db, insert_feed_to_db};
use bytes::Bytes;
use log::{error, trace};
use rss::{Channel, Item};
use rusqlite::Connection;
use std::error;

/// **Purpose**:    Grab new items from an rss feed and maintains DB feed history
/// **Parameters**: A &rusqlite:Connection of the history db, a &String of the feed url, A Bytes object of rss content
/// **Ok Return**:  A Vec<rss::Item> of previously unseen rss content
/// **Err Return**: A rss::Error from failure to serialize a rss::Channel
/// **Panics**:     No
/// **Modifies**:   If new rss::Items are found, updates the DB row for the feed
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn get_new_rss_items(
    conn: &Connection,
    feed_url: &String,
    feed_bytes: Bytes,
) -> Result<Vec<Item>, Box<dyn error::Error>> {
    trace!("Inside get_new_rss_items.");

    let db_feed_items: Vec<Item> = match get_feed_from_db(conn, feed_url) {
        Ok(response) => match serde_json::from_str(response.history.as_str()) {
            Ok(items) => items,
            Err(err) => {
                error!("Failed to turn DB feed history string into Vec<rss:Item>.");
                return Err(Box::new(err));
            }
        },
        Err(err) => {
            error!("Failed to get {} row from DB.", feed_url);
            return Err(Box::new(err));
        }
    };

    let new_feed_channel: Channel = match Channel::read_from(&feed_bytes[..]) {
        Ok(channel) => channel,
        Err(err) => {
            error!("Failed to convert feed bytes to rss channel.");
            return Err(Box::new(err));
        }
    };

    let new_items: Vec<Item> = make_new_item_vector(db_feed_items, new_feed_channel.clone());

    // if there are differences, update db row and return the new items
    if !new_items.is_empty() {
        let updated_row: DBEntry = DBEntry {
            feed_name: feed_url.clone(),
            history: stringify_feed_bytes(feed_bytes),
            last_modified: None,
            etag: None,
        };

        // technically we don't have to return the error here, but it makes more sense to.
        // if we can't update here, we will find the changes from this iteration again next time
        // as it will still be comparing with the old DB data
        match insert_feed_to_db(conn, updated_row) {
            Ok(_) => {}
            Err(err) => {
                error!("Could not update DB row for {}.", feed_url);
                return Err(Box::new(err));
            }
        }
    }

    Ok(new_items)
}

/// **Purpose**:    Serialize a Bytes object into a String
/// **Parameters**: A Bytes object of rss content
/// **Return**:     A String of rss content
/// **Panics**:     If conversion of Bytes to rss Channel or rss Channel to String fails
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Completed, but need to change to return errors and change to only serialize urls
/// and titles instead of the whole item
pub fn stringify_feed_bytes(feed_bytes: Bytes) -> String {
    trace!("Inside serialize_feed_bytes");
    let rss_channel: Channel = match Channel::read_from(&feed_bytes[..]) {
        // TODO Change to return errors
        Ok(result) => result,
        Err(err) => {
            error!(
                "Couldn't convert bytes to rss Channel due to error: {}.",
                err
            );
            panic!();
        }
    };
    // I am concerend about serializing the whole item because the description tag may have
    // issues with a date-dependent element changing from pull to pull.
    let serialized: String = match serde_json::to_string(&rss_channel.items().to_vec()) {
        // TODO This is a candidate to return the error
        Ok(json) => json,
        Err(err) => {
            error!("Couldn't serialize item vector! {}", err);
            panic!();
        }
    };

    serialized
}

/// **Purpose**:    Constructs a vector of previously unseen rss content
/// **Parameters**: A Vec<rss:Item> of previously seen rss content
///                 A rss::Channel of a all content on an rss feed
/// **Returns**:    A Vec<rss::Item> of rss content in the passed Channel that is not in the passed Vec
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn make_new_item_vector(old_items: Vec<Item>, new_channel: Channel) -> Vec<Item> {
    trace!(
        "Inside make_new_item_vector with {} old_items.",
        old_items.len()
    );

    let mut new_items: Vec<Item> = Vec::new();

    for item in new_channel.items().iter() {
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
pub fn print_serialized_rss(items: Vec<Item>) {
    trace!("Inside print_serialized_rss with  {} items.", items.len());
    if !items.is_empty() {
        for item in items.iter() {
            if let Some(thing) = item.title() {
                println!("TITLE IS: {}", thing);
            }
            if let Some(thing) = item.link() {
                println!("LINK IS: {}", thing);
            }
            if let Some(thing) = item.description() {
                println!("DESC IS: {}", thing);
            }
            if let Some(thing) = item.author() {
                println!("AUTHOR IS: {}", thing);
            }
            if !item.categories().is_empty() {
                for category in item.categories().iter() {
                    println!("CATEGORY NAME IS: {}", category.name());
                    if let Some(thing) = category.domain() {
                        println!("CATEGORY DOMAIN IS: {}", thing);
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
                println!("COMMENTS IS: {}", thing);
            }
            if let Some(thing) = item.pub_date() {
                println!("PUB DATE IS: {}", thing);
            }
            if let Some(thing) = item.content() {
                println!("CONTENT IS: {}", thing);
            }
        }
    }
}

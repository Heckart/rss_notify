use crate::env_setup::source_env_var;
use bytes::Bytes;
use log::{debug, error, info, trace};
use rss::{Channel, Error, Item};
use std::fs::{File, create_dir, write};
use std::io::BufReader;
use std::path::Path;

/// **Purpose**:    Grab new items from an rss feed and maintains feed history, or create a feed history file if it doesn't
///                 already exist
/// **Parameters**: A Bytes object of rss content
/// **Ok Return**:  A Vec<rss::Item> of previously unseen rss content
/// **Err Return**: A rss::Error from failure to serialize a rss::Channel
/// **Panics**:     If rss json cannot be serialized or deserialized, hist dir cannot be opened or
///                 created, hist file cannot be opened, read, or written to
/// **Modifies**:   Creates a hist file in the hist dir if it doesn't already exist. If it does exist,
///                 replaces the existing hist file with the newest feed content
/// **Tests**:      Not implemented yet
/// **Status**:     Functional, but hist_file_name can be improved as can the serialization, can
///                 also be broken up and made more modular so its easier to read
pub fn get_new_rss_items(feed_bytes: Bytes) -> Result<Vec<Item>, Error> {
    trace!("Inside get_new_rss_items.");
    let hist_dir: String = source_env_var("RSS_NOTIFY_HISTORY_DIR");
    let rss_channel: Channel = serialize_rss(feed_bytes)?;
    // TODO:Change this to be based on the rss feed url itself
    let hist_file_name: String = rss_channel.title().replace(" ", "").replace(".", "");
    let hist_file_path = hist_dir.clone() + "/" + &hist_file_name + ".hist";

    // I am concerend about serializing the whole item because the description tag may have
    // issues with a date-dependent element changing from pull to pull.
    let serialized: String = match serde_json::to_string(&rss_channel.items().to_vec()) {
        // TODO This is a candidate to return the error
        Ok(json) => {
            trace!("serialized to json.");
            json
        }
        Err(err) => {
            error!("Couldn't serialize item vector! {}", err);
            panic!();
        }
    };

    if !Path::new(&hist_dir).exists() {
        match create_dir(&hist_dir) {
            Ok(_) => {
                info!("Hist dir {} did not exist, so created it.", hist_dir);
            }
            Err(err) => {
                error!("Could not create hist dir! {}", err);
                panic!();
            }
        }
    }

    if !Path::new(&hist_file_path).exists() {
        // if the hist file doesn't exist, create and populate it, but return an empty vec
        match File::create(&hist_file_path) {
            Ok(_) => {
                info!("Hist file {} did not exist, so created it.", hist_file_path);
            }
            Err(err) => {
                error!("Couldn't create feed history file! {}", err);
                panic!();
            }
        };

        match write(hist_file_path, serialized) {
            Ok(_) => {
                trace!("Wrote history to hist file");
            }
            Err(err) => {
                error!("Couldn't write item vec to hist file! {}", err);
                panic!();
            }
        }

        let empty_vec: Vec<Item> = Vec::new();
        Ok(empty_vec)
    } else {
        // if the hist file exists, grab the saved history, compare the new items with the
        // historical items, if something is in the new list not in the old list, add it to the return
        // vector.
        // Replace the hist file contents with the items in the rss_channel.
        let hist_file: File = match File::open(&hist_file_path) {
            Ok(f) => {
                trace!("Hist file exists, so opened it.");
                f
            }
            Err(err) => {
                error!("Could not open hist file! {}", err);
                panic!();
            }
        };
        let file_read = BufReader::new(hist_file);

        let hist_items: Vec<Item> = match serde_json::from_reader(file_read) {
            Ok(items) => {
                trace!("Built item vector of history items.");
                items
            }
            Err(err) => {
                error!("Couldn't deserialize hist file content! {}", err);
                panic!();
            }
        };

        let new_items: Vec<Item> = make_new_item_vector(hist_items, rss_channel.clone());

        if !new_items.is_empty() {
            // need to replace hist file contents with the channel contents
            match write(hist_file_path, serialized) {
                Ok(_) => {
                    debug!("Wrote to hist file.");
                }
                Err(err) => {
                    error!("Couldn't write item vec to hist file! {}", err);
                    panic!();
                }
            }
        }
        Ok(new_items)
    }
}

/// **Purpose**:    Serialize a Bytes object into an rss::Channel
/// **Parameters**: A Bytes object of rss content
/// **Ok Return**:  A rss::Channel of rss content
/// **Err Return**: A rss::Error from not being able to read an rss::Channel
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn serialize_rss(feed_bytes: Bytes) -> Result<Channel, Error> {
    trace!("Inside serialize_rss.");
    let rss_object: Channel = Channel::read_from(&feed_bytes[..])?;

    Ok(rss_object)
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
            trace!("Found new rss item.");
            new_items.push(item.clone());
        }
    }

    new_items
}

/*
// This function is an absolute unmitigated disaster full of stupid non-idiomatic hacks!
// Dates are highly feed specific.
// This function parses the format: "Sun, 30 Nov 2025 09:30:00 +0100"
// yyyymmddhhmmss will be stored as an int for easier comparison later
// TODO: At some point this system can be hugely improved
fn parse_date(item: rss::Item) -> i64 {
    let correct_date_format_len: usize = 6;

    let feed_date_string: &str = match item.pub_date() {
        Some(date) => date,
        None => {
            eprintln!("Item has no date.");
            return -1;
        }
    };

    let date_elements: Vec<&str> = feed_date_string.split_whitespace().collect();
    if date_elements.len() != correct_date_format_len {
        eprintln!("Wrong date format.");
        return -1;
    }

    // we will calculate from the bottom up, first the offset, then time, then date, month, year
    // it is a huge PITA to access chars by index in a str in rust
    let mut min_offset: i16 = match date_elements.get(5).and_then(|s| s.get(1..3)) {
        Some(num) => match num.parse() {
            Ok(int) => int,
            Err(err) => {
                eprintln!("Couldn't parse min offset '{}': {}", num, err);
                0
            }
        },
        None => {
            eprintln!("min offset missing?");
            0
        }
    };

    let mut hour_offset: i16 = match date_elements.get(5).and_then(|s| s.get(1..3)) {
        Some(num) => match num.parse() {
            Ok(int) => int,
            Err(err) => {
                eprintln!("Couldn't parse hour offset '{}': {}", num, err);
                0
            }
        },
        None => {
            eprintln!("Hour offset missing?");
            0
        }
    };

    // a bit faster than using .starts_with()
    // works because we know it will always be valid ascii
    if date_elements[5].as_bytes().first() == Some(&b'-') {
        hour_offset = -hour_offset;
        min_offset = -min_offset;
    }

    let seconds: i16 = match date_elements.get(4).and_then(|s| s.get(6..)) {
        Some(num) => match num.parse() {
            Ok(int) => int,
            Err(err) => {
                eprintln!("Couldn't parse seconds '{}': {}", num, err);
                0
            }
        },
        None => {
            eprintln!("seconds missing?");
            0
        }
    };
    let mut seconds_str: String = seconds.to_string();
    if seconds_str.len() < 2 {
        seconds_str = "0".to_owned() + &seconds_str;
    }

    let mut minutes: i16 = match date_elements.get(4).and_then(|s| s.get(3..5)) {
        Some(num) => match num.parse() {
            Ok(int) => int,
            Err(err) => {
                eprintln!("Couldn't parse minutes '{}': {}", num, err);
                0
            }
        },
        None => {
            eprintln!("minutes missing?");
            0
        }
    };
    // apply minute offset and perform necessary calculations
    minutes += min_offset;
    assert!(minutes < 159);
    hour_offset += minutes / 60;
    assert!(hour_offset < 3);
    minutes %= 60;
    assert!(minutes < 60);
    let mut minutes_str: String = minutes.to_string();
    if minutes_str.len() < 2 {
        minutes_str = "0".to_owned() + &minutes_str;
    }

    let mut hours: i16 = match date_elements.get(4).and_then(|s| s.get(0..3)) {
        Some(num) => match num.parse() {
            Ok(int) => int,
            Err(err) => {
                eprintln!("Couldn't parse hours '{}': {}", num, err);
                0
            }
        },
        None => {
            eprintln!("hours missing?");
            0
        }
    };
    hours += hour_offset;
    assert!(hours < 125);
    let mut day_offset: i16 = 0;
    if hours > 23 {
        day_offset = hours / 23;
        assert!(day_offset < 6);
        hours %= 23;
    }
    assert!(hours < 24);
    let mut hours_str: String = hours.to_string();
    if hours_str.len() < 2 {
        hours_str = "0".to_owned() + &hours_str;
    }

    let mut days: i16 = match date_elements.get(1) {
        Some(num) => match num.parse() {
            Ok(int) => int,
            Err(err) => {
                eprintln!("Couldn't parse days '{}': {}", num, err);
                0
            }
        },
        None => {
            eprintln!("days missing?");
            0
        }
    };
    days += day_offset;
    assert!(days < 37);
    // This logic should depend on the month in theory but this is just going to stay
    // like this for now and probably forever
    let mut month_offset: i16 = 0;
    if days > 31 {
        month_offset = 1;
        days = 31;
    }
    let mut days_str: String = days.to_string();
    if days_str.len() < 2 {
        days_str = "0".to_owned() + &days_str;
    }

    let mut month: i16 = match date_elements.get(2) {
        Some(mnth) => match *mnth {
            "Jan" => 1,
            "Feb" => 2,
            "Mar" => 3,
            "Apr" => 4,
            "May" => 5,
            "Jun" => 6,
            "Jul" => 7,
            "Aug" => 8,
            "Sep" => 9,
            "Oct" => 10,
            "Nov" => 11,
            "Dec" => 12,
            _ => {
                eprintln!("Unkown month? {}", mnth);
                0
            }
        },
        None => {
            eprintln!("month missing?");
            0
        }
    };
    month += month_offset;
    assert!(month < 14);
    let mut year_offset: i16 = 0;
    if month > 12 {
        year_offset = 1;
        month = 1;
    }
    let mut month_str: String = month.to_string();
    if month_str.len() < 2 {
        month_str = "0".to_owned() + &month_str;
    }

    let mut year: i16 = match date_elements.get(3) {
        Some(num) => match num.parse() {
            Ok(int) => int,
            Err(err) => {
                eprintln!("Couldn't parse year '{}': {}", num, err);
                0
            }
        },
        None => {
            eprintln!("year missing?");
            0
        }
    };
    year += year_offset;
    let mut year_str: String = year.to_string();
    if year_str.len() < 2 {
        year_str = "0".to_owned() + &year_str;
    }

    let timestamp: i64 = match (year_str
        + &month_str
        + &days_str
        + &hours_str
        + &minutes_str
        + &seconds_str)
        .parse()
    {
        Ok(num) => num,
        Err(err) => {
            eprintln!("Could not construct timestamp. {}", err);
            0
        }
    };

    timestamp
}
*/

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

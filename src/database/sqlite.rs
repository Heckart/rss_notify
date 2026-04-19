use crate::{database, env_setup::source_env_var};
use log::{debug, error, trace};
use rusqlite::{Connection, params};

pub struct DBEntry {
    pub feed_name: String,
    pub history: String,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
}

/// **Purpose**:    Create the db and table if it does not exist and provide the db connection object
/// **Parameters**: A &str with the name of an env variable containing the full path and name of a db
/// **Returns**:    A rusqlite::Connection for the sqlite database
/// **Panics**:     If a connection cannot be made to the database
/// **Modifies**:   Creates a sqlite db if it doesn't exist
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn setup_db(db_name: &str) -> Connection {
    trace!("Inside setup_db.");

    let conn: Connection = match Connection::open(source_env_var(db_name)) {
        Ok(connection) => {
            debug!("{} DB connection established.", db_name);
            connection
        }
        Err(err) => {
            error!("Could not setup database connection due to error: {err}.");
            panic!();
        }
    };

    initialize_feed_table(&conn);

    conn
}

/// **Purpose**:    Creates the feed history table if it doesn't exist
/// **Parameters**: A &rusqlite::Connection for the database
/// **Returns**:    Nothing
/// **Panics**:     If the new table cannot be made
/// **Modifies**:   Creates the feed history table in the sqlite db if it doesn't exist
/// **Tests**:      Not implemented yet
/// **Status**:     Done
fn initialize_feed_table(conn: &Connection) {
    trace!("Inside initialize_feed_table.");
    match conn.execute(
        "CREATE TABLE IF NOT EXISTS feed_hist (
            feed_name       TEXT PRIMARY KEY,
            history         TEXT NOT NULL,
            last_modified   TEXT,
            etag            TEXT
        )",
        (),
    ) {
        Ok(_) => {
            trace!("feed_hist table initialized.");
        }
        Err(err) => {
            error!("CREATE TABLE responded with err {}.", err);
            panic!();
        }
    };
}

/// **Purpose**:    Finds if a specific feed exists in the feed table
/// **Parameters**: A &rusqlite::Connection for the database, a &String with a feed name
/// **Ok Return**:  A boolean representing whether or not the feed is in the table
/// **Err Return**: A rusqlite::Error from not being able to determine the existence of the feed
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn feed_is_in_db(conn: &Connection, feed: &String) -> Result<bool, rusqlite::Error> {
    trace!("Inside feed_is_in_db searching for existence of {}.", feed);
    match conn.query_one(
        "SELECT COUNT(1) 
        FROM feed_hist 
        WHERE feed_name = ?1",
        params!(feed),
        |row| row.get::<_, i64>(0),
    ) {
        Ok(count) => {
            if count > 0 {
                debug!("{} exists in feed_hist.", feed);
                Ok(true)
            } else {
                debug!("{} does not exist in feed_hist.", feed);
                Ok(false)
            }
        }
        Err(err) => {
            error!(
                "Could not determine if {} is in feed_hist due to error: {}.",
                feed, err
            );
            Err(err)
        }
    }
}

/// **Purpose**:    Returns the row of a specific feed that should exist in the table
/// **Parameters**: A &rusqlite::Connection for the database, a &str with a feed name
/// **Ok Return**:  A database::sqlite::DBEntry of all rows from the feed
/// **Err Return**: A rusqlite::Error from the query failing or row not existing
/// **Panics**:     No
/// **Modifies**:   Nothing
/// **Tests**:      Not implemented yet
/// **Status**:     Done
pub fn get_feed_from_db(
    conn: &Connection,
    feed: &str,
) -> Result<database::sqlite::DBEntry, rusqlite::Error> {
    trace!("Inside get_feed_hist_from_db getting hist for {}.", feed);

    let row_content: Result<DBEntry, rusqlite::Error> = match conn.query_row(
        "SELECT feed_name, history, last_modified, etag 
        FROM feed_hist 
        WHERE feed_name = ?1",
        params!(feed),
        |row| {
            Ok(DBEntry {
                feed_name: match row.get(0) {
                    Ok(ok) => {
                        trace!("DB feed_name {} extracted.", ok);
                        ok
                    }
                    Err(err) => {
                        error!("Could not get feed_name row due to error: {}.", err);
                        return Err(err);
                    }
                },
                history: match row.get(1) {
                    Ok(ok) => {
                        trace!("DB history extracted.");
                        ok
                    }
                    Err(err) => {
                        error!("Could not get history row due to error: {}.", err);
                        return Err(err);
                    }
                },
                last_modified: match row.get(2) {
                    Ok(ok) => {
                        trace!("DB last_modified extracted.");
                        ok
                    }
                    Err(err) => {
                        error!("Could not get last_modified row due to error: {}.", err);
                        return Err(err);
                    }
                },
                etag: match row.get(3) {
                    Ok(ok) => {
                        trace!("DB etag extracted.");
                        ok
                    }
                    Err(err) => {
                        error!("Could not get etag row due to error: {}.", err);
                        return Err(err);
                    }
                },
            })
        },
    ) {
        Ok(entry) => {
            debug!("Grabbed entry with feed_name {}.", entry.feed_name);
            Ok(entry)
        }
        Err(err) => {
            error!("Query for {} failed with error: {}.", feed, err);
            Err(err)
        }
    };

    row_content
}

/// **Purpose**:    Creates or updates a row for a feed
/// **Parameters**: A &rusqlite::Connection for the database, a database::sqlite::DBEntry with new
///                 row contents
/// **Ok Return**:  A usize representing a success status code
/// **Err Return**: A rusqlite::Error from the query failing
/// **Panics**:     No
/// **Modifies**:   Creates or updates a row in the db with the new_row's feed_name column
/// **Tests**:      Not implemented yet
/// **Status**:     Done.
pub fn insert_feed_to_db(conn: &Connection, new_row: DBEntry) -> Result<usize, rusqlite::Error> {
    trace!(
        "Inside insert_feed_to_db inserting feed {}.",
        new_row.feed_name
    );

    match conn.execute(
        "INSERT INTO feed_hist (feed_name, history, last_modified, etag)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(feed_name) DO UPDATE SET
            history = ?2,
            last_modified = ?3,
            etag = ?4
            ",
        params!(
            new_row.feed_name,
            new_row.history,
            new_row.last_modified,
            new_row.etag
        ),
    ) {
        Ok(ok) => {
            debug!("Insert query updated {} rows.", ok);
            Ok(ok)
        }
        Err(err) => {
            error!("Insert query responeded with error: {}.", err);
            Err(err)
        }
    }
}

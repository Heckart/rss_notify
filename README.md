# RSS Notify
Artisanal, hand-crafted, agent-free code to automatically observe rss feeds and send push notifications whenever new items are present.

## Inspiration
In the past, I have tried to find a service that would allow me to receive notifications whenever an rss feed gets new items. Most rss reader applications do not have such functionality and the few that do offer it as a paid service. Despite that, this is a fairly simple functionality to implement. In 2022, as a sophomore in college, I did it myself in Python. I was not a very good programmer back then; the code was messy, full of bad practices, difficult to understand, and had some bugs that proved very difficult to track down. This project is a from-the-ground-up rewrite of that original Python script which I intend to be significantly better, less error prone (and more resilient when encountering errors), and to generally be written with best practices in mind. I currently have it up and running on a Rasperry Pi 4 Model B.

## Building and Running
1. This is a rust project, so you should have the rust toolchain installed. You can follow the official instructions [here](https://rust-lang.org/tools/install/).
2. Once you have cargo available from the previous step, run `cargo build --release` to compile the binary. This may take a couple minutes if you have a slow processor.
3. Create a file with the rss links you’d like to track. The only rules are that it must be a valid rss feed and you must put only one feed url per line in the file. The file can be called whatever you want and be stored wherever you’d like on your system (more info in the next step).
4. This project uses environment variables for all system-specific configurations and for storing secrets. There are six variables that must be present in a `.env` file in the same directory as the provided `exec_rss_notify.sh` script. A sample `.env` file is provided below:
```bash
export RSS_NOTIFY_DB="/absolute/path/to/db/name.sql" # what you want your db to be called and where it should be stored
export RSS_NOTIFY_FEED_LIST="/abolute/path/to/feed/list/from/step/three/feeds.rss" # where you are storing your feed file
export RSS_NOTIFY_BIN="/absolute/path/to/rss_notify/target/release/rss_notify" # the path to the compiled binary. If you followed step two, it will be inside the rss_notify dir as shown in this sample
cur_date=$(date +%Y%m%d%H%M%S) # optional, just used to put the date in log files
export RSS_NOTIFY_LOG_FILE="/absolute/path/to/where/you/want/rss_notify_log_${cur_date}.log" # wherever you want your log files stored and the naming convention for the individual log files
export RSS_NOTIFY_LOG_RETENTION_DAYS=3 # the number of days you want log files to be saved for
export NTFY_TOPIC="sampleTopic" # the ntfy topic you want to publish new rss items to
export RUST_LOG="info" # optional, whatever logging level you want (trace, debug, info, warn, error)
```
5. As mentioned in the sample `.env` file, [ntfy](https://docs.ntfy.sh/) is used for the push notifications. You will be able to send 250 pushes per day on the public API. This project originally used Pushbullet, but was switched to ntfy because ntfy is open source and has *significantly* higher usage limits.
6. You’re ready to run rss notify! I suggest running the provided `exec_rss_notify.sh` orchestrator script to do this, as it will automatically handle things like log creation and cleanup for you. You will not receive any notifications at first, as the first time a is read its history is only saved, but if you keep the program running (or kill it and start it up later without deleting the feed hist) as a new item makes it to the feed, you will get a ntfy notification from rss notify. You can install the ntfy client to your computer or phone.
7. Right now, rss notify is intended to be run in the background via cron so that it is always keeping up to date with the state of your tracked feeds, without needing any manual interventions. `exec_rss_notify.sh` will kill all other running instances of rss notify when it run, so I am currently using this cron schedule, which will fire once per day at midnight:
```cron
0 0 * * * /absolute/path/to/rss_notify/exec_rss_notify.sh >> /absolute/path/to/rss_notify/logs/exec_rss_notify$(date +\%Y\%m\%d\%H\%M\%S).cron
```

## Control Flow
### The program will enter an infinite loop where the following steps are repeated for every feed url:
1. A GET request is made to the feed url, capturing its contents as bytes.
2. The byte content is converted into rss items.
3. If a feed has never been downloaded before, its DB entry with current feed contents is created and no more processing is done for the feed in the current loop.
4. If a feed has an DB entry, the current feed contents are compared with the DB Entry.
5. If any new items exist, a POST request is sent to the ntfy API to alert about the new item, sending the item title and url.
### Error handling
If any of the steps mentioned above encounter an error, the program adds them to an error vector and will attempt to send a ntfy push containing information on the encountered errors. The set-it-and-forget-it nature of this script means we will largely avoid panics. Currently, the script only panics if env variables cannot be read, db connection can't be establish, table can't be created, or if an rss feed is unserializable.

## Styles and Standards
### General
1. Functions and variables should have descriptive names and use snake_case.
2. Functions should have cyclomatic complexity <= 16.
### Bash
1. Code should pass all [shellcheck](https://github.com/koalaman/shellcheck) static analysis rules.
2. All variables should be double quoted and braced when being referenced.
2. Code should be formatted with default [shfmt](https://github.com/mvdan/sh) rules.
### Rust
1. Code should pass all [clippy](https://github.com/rust-lang/rust-clippy) lint rules.
2. All variables and functions should be explicitly typed.
3. All functions should be commented with the following style:
```rust
/// **Purpose**:    A brief description of what the function does
/// **Parameters**: A list of all parameters, their type, and purpose
/// **Ok Return**:  Description of the type and contents of the ‘Ok’ return of a Result return type 
/// **Err Return**: Description of the type, contents, and cause of the ‘Err’ return of a Result return type
/// **Return** (IF NOT A RESULT TYPE): Description of the type and contents of a function’s return 
/// **Panics**:     If the function panics, state what causes it to panic
/// **Modifies**:   If the function modifies any variables or parts of the system, state what and why
/// **Tests**:      Link to the closest parent directory of any tests, if they exist
/// **Status**:     ‘Done’ if no additional work is expected for a function, else list what else there is to do
```
4. Use `use` statements to the largest extent possible.
5. Code should be formatted with [rustfmt](https://github.com/rust-lang/rustfmt).

## TODO
Here is some of the work that I still want to do, in no particular order:
1. Add the ability to track changes to websites in general, rather than just rss feeds.
2. Start using `etag`s or the `last-modified` header to decide whether or not to pull down a feed's contents in the first place, rather than actually pulling down the whole thing every time. (SOON)
3. Possibly make the GET and POST requests async instead of blocking, though I’m not sure if the effort is worth it on this one.
4. Implement unit, integration, and end-to-end tests for everything.
5. Set a max size for the error vector. If the number of encountered, unalerted errors goes over the limit, just kill the program to avoid potentially using up the entirety of free ntfy push capacity once the error pushes are allowed to go through. Possibly also start tracking error rate over time and even if the error pushes go through, but an earlier step is erroring on every loop, then also end early.
6. Set up a CI pipeline to enforce the linting and formatting rules specified in the above section.
7. There are a few `panic!()`s that can probably be changed to more graceful error handling.
8. `get_new_rss_items` can be broken up and made more modular.
9. Eventually, shift away from a third party service (ntfy) and create a basic Android shell app that leverages FCM to handle notifications.

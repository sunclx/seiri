#![feature(fs_read_write)]
#![feature(toowned_clone_into)]
#![feature(mpsc_select)]
#![feature(ascii_ctype)]


#[cfg(feature = "use_graphql")]
#[macro_use]
extern crate juniper;
#[cfg(feature = "use_graphql")]
extern crate juniper_rocket;
#[cfg(feature = "use_graphql")]
extern crate rayon;
#[cfg(feature = "use_graphql")]
extern crate rocket;
#[cfg(feature = "use_graphql")]
extern crate rocket_cors;
#[cfg(feature = "use_graphql")]
mod graphql;
#[cfg(feature = "use_graphql")]
use juniper::EmptyMutation;
#[cfg(feature = "use_graphql")]
use rocket::config::Environment;
#[cfg(feature = "use_graphql")]
use rocket::http::Method;
#[cfg(feature = "use_graphql")]
use rocket::response::content;
#[cfg(feature = "use_graphql")]
use rocket::Config as RocketConfig;
#[cfg(feature = "use_graphql")]
use rocket::State;
#[cfg(feature = "use_graphql")]
use rocket_cors::{AllowedHeaders, AllowedOrigins};

extern crate seiri;
extern crate walkdir;
extern crate notify;

use std::io;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

mod utils;
mod watcher;

use seiri::config::Config;
use seiri::config;
use seiri::database;
use seiri::database::Connection;
use seiri::database::ConnectionPool;
use seiri::paths;
use seiri::Error;
use seiri::Track;
use seiri::TaglibTrack;
use watcher::WatchStatus;

fn process(path: &Path, config: &Config, conn: &Connection) {
    let track = Track::from_taglibsharp(path, None);
    match track {
        Ok(track) => match paths::ensure_music_folder(&config.music_folder) {
            Ok(library_path) => {
                let track = paths::move_new_track(&track, &library_path.0, &library_path.1);
                if let Ok(track) = track {
                    database::add_track(&track, conn);
                    eprintln!("TRACKADDED~{:?}:Added {:?} to database", track.title, track);
                }
            }
            Err(_) => eprintln!("LIBRARYNOTFOUND~The library path was not found."),
        },
        Err(err) => match err {
            Error::UnsupportedFile(file_name) => {
                match paths::ensure_music_folder(&config.music_folder) {
                    Ok(library_path) => {
                        paths::move_non_track(&file_name, &library_path.1).unwrap();
                        eprintln!(
                            "NONTRACK~{:?}:Found and moved non-track item {:?}",
                            file_name, file_name
                        )
                    }
                    Err(err) => eprintln!(
                        "TRACKMOVEERROR~{:?}:Error {} ocurred when attempting to move track.",
                        file_name, err
                    ),
                };
            }
            Error::MissingRequiredTag(file_name, tag) => eprintln!(
                "MISSINGTAG~Found track {} but missing tag {}.",
                file_name, tag
            ),
            Error::HelperNotFound => eprintln!("HELPERNOTFOUND~Katatsuki TagLib helper not found."),
            _ => {}
        },
    }
}

fn wait_for_watch_root_available(folder: &str) -> (PathBuf, PathBuf) {
    println!("Waiting for folder {}...", folder);
    let wait_time = Duration::from_secs(5);
    while let Err(_) = paths::ensure_music_folder(folder) {
        thread::park_timeout(wait_time);
    }
    println!("Successfully ensured folder {}", folder);
    paths::ensure_music_folder(folder).unwrap()
}

fn begin_watch(config: &Config, pool: &ConnectionPool, rx: Receiver<WatchStatus>) {
    let auto_paths = wait_for_watch_root_available(&config.music_folder);
    let watch_path = &auto_paths.1.to_str().unwrap();
    println!("Watching {}", watch_path);
    watcher::list(&watch_path, &config, &pool, process);
    // Create a channel to receive the events.
    if let Err(e) = watcher::watch(&watch_path, &config, &pool, process, rx) {
        println!("{}", e);
    }
}

fn get_watcher_thread(rx: Receiver<WatchStatus>) -> io::Result<thread::JoinHandle<()>> {
    thread::Builder::new()
        .name("WatchThread".to_string())
        .spawn(move || {
            let config = config::get_config();
            let pool = database::get_connection_pool();
            begin_watch(&config, &pool, rx)
        })
}

fn start_watcher_watchdog(wait_time: Duration) {
    thread::spawn(move || {
        let (tx, rx) = channel();
        let mut tx = tx;
        let config = config::get_config();
        wait_for_watch_root_available(&config.music_folder);
        let mut _watch_thread = get_watcher_thread(rx).unwrap();
        loop {
            thread::park_timeout(wait_time);
            if let Err(_) = tx.send(WatchStatus::KeepAlive) {
                eprintln!("WATCHERKEEPALIVEFAIL~Keep-alive failed. Watcher thread probably panicked. Restarting Watcher Thread...");
                let (new_tx, rx) = channel();
                tx = new_tx.clone();
                _watch_thread = get_watcher_thread(rx).unwrap();
            }

            let music_folder = paths::ensure_music_folder(&config.music_folder);
            if let Err(_) = music_folder {
                eprintln!(
                    "WATCHERFOLDERACCESSLOST~Lost access to {}",
                    &config.music_folder
                );
                wait_for_watch_root_available(&config.music_folder);
                let (new_tx, rx) = channel();
                tx.send(WatchStatus::Exit).unwrap();
                eprintln!(
                    "WATCHERRESTART~Requested watcher thread exit. Restarting Watcher Thread..."
                );
                tx = new_tx.clone();
                _watch_thread = get_watcher_thread(rx).unwrap();
            }
        }
    });
}

fn ensure_port(port: u16) -> Result<TcpListener, io::Error> {
    match TcpListener::bind(("localhost", port)) {
        Ok(socket) => Ok(socket),
        Err(err) => Err(err),
    }
}

#[cfg(feature = "use_graphql")]
#[get("/")]
fn graphiql() -> content::Html<String> {
    juniper_rocket::graphiql_source("/graphql")
}

#[cfg(feature = "use_graphql")]
type Schema = juniper::RootNode<'static, graphql::Query, EmptyMutation<graphql::Context>>;

#[cfg(feature = "use_graphql")]
#[post("/graphql", data = "<request>")]
fn post_graphql_handler(
    context: State<graphql::Context>,
    request: juniper_rocket::GraphQLRequest,
    schema: State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&schema, &context)
}

#[cfg(feature = "use_graphql")]
fn start_rocket() {
    let options = rocket_cors::Cors {
        allowed_origins: AllowedOrigins::all(),
        allowed_methods: vec![Method::Get, Method::Post]
            .into_iter()
            .map(From::from)
            .collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        ..Default::default()
    };
    thread::spawn(move || {
        let config = RocketConfig::build(Environment::Development)
            .address("localhost")
            .port(9234)
            .finalize()
            .unwrap();
        rocket::custom(config, true)
            .manage(graphql::Context::new())
            .manage(Schema::new(
                graphql::Query::new(),
                EmptyMutation::<graphql::Context>::new(),
            ))
            .mount("/", routes![graphiql, post_graphql_handler])
            .attach(options)
            .launch();
    });
}

fn main() {
    let _lock = ensure_port(9235).expect("Unable to acquire lock");

    let wait_time = Duration::from_secs(5);
    start_watcher_watchdog(wait_time);

    #[cfg(feature = "use_graphql")]
    {
        start_rocket();
    }

    let conn = database::get_database_connection();
    utils::wait_for_exit(&conn);
}
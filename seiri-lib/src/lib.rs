#![feature(fs_read_write)]
#![feature(toowned_clone_into)]
#![feature(ascii_ctype)]

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate quick_error;

extern crate chrono;
extern crate humantime;
extern crate itertools;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate rand;
extern crate regex;
extern crate rusqlite;
extern crate app_dirs;
extern crate toml;
extern crate katatsuki;

mod bangs;
mod error;


pub use katatsuki::TrackFileType;
pub use katatsuki::Track;
pub use error::{Error, Result};
pub use bangs::Bang;

pub mod config;
pub mod database;
pub mod paths;

pub mod ticks {
    pub use bangs::ms_to_ticks;
    pub use bangs::ticks_to_ms;
}
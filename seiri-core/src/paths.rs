use app_dirs::*;
use chrono::prelude::*;
use r2d2::{CustomizeConnection, Pool};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, Error as SqliteError, Result as SqliteResult};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use seiri::{Error, Result, Track};
use seiri::database::{add_regexp_function, create_database, enable_wal_mode};
use track::TaglibTrack;
use std::ascii::AsciiExt;

trait InvalidChar {
    fn is_invalid_for_path(&self) -> bool;
}

impl InvalidChar for char {
    fn is_invalid_for_path(&self) -> bool {
        match *self {
            '\"' | '<' | '>' | '|' | '\0' | ':' | '*' | '?' | '\\' | '/' => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct SeiriConnectionCustomizer;
impl CustomizeConnection<Connection, SqliteError> for SeiriConnectionCustomizer {
    fn on_acquire(&self, conn: &mut Connection) -> SqliteResult<()> {
        enable_wal_mode(conn).unwrap();
        add_regexp_function(conn).unwrap();
        create_database(conn);
        Ok(())
    }
}

/// Gets the application data path.
/// Panics if unable to be created.
pub fn get_appdata_path() -> PathBuf {
    let appdata_path = get_data_root(AppDataType::UserConfig)
        .ok()
        .and_then(|mut p: PathBuf| {
            p.push(".seiri");
            Some(p)
        })
        .unwrap();
    if let Err(_) = fs::create_dir_all(appdata_path.as_path()) {
        panic!(
            "Unable to create application directory at {:?}",
            appdata_path
        )
    }
    appdata_path
}

pub fn get_database_connection() -> Connection {
    let mut database_path = get_appdata_path();
    database_path.push("tracks.db");
    let conn = Connection::open(database_path.as_path()).unwrap();
    enable_wal_mode(&conn).unwrap();
    add_regexp_function(&conn).unwrap();
    create_database(&conn);
    conn
}

pub fn get_connection_pool() -> Pool<SqliteConnectionManager> {
    let mut database_path = get_appdata_path();
    database_path.push("tracks.db");
    let manager = SqliteConnectionManager::file(&database_path);
    let pool = Pool::builder()
        .connection_customizer(Box::new(SeiriConnectionCustomizer))
        .build(manager)
        .unwrap();
    pool
}

pub fn ensure_music_folder(folder_path: &str) -> io::Result<(PathBuf, PathBuf)> {
    // Todo: handle these unwraps properly.
    let music_folder = Path::new(folder_path);
    let music_folder = PathBuf::from(music_folder);
    let mut auto_add_folder = PathBuf::new();
    music_folder.clone_into(&mut auto_add_folder);
    auto_add_folder.pop();
    auto_add_folder.push("Automatically Add to Library");
    fs::create_dir_all(music_folder.as_path())?;
    fs::create_dir_all(auto_add_folder.as_path())?;
    Ok((music_folder, auto_add_folder))
}

fn sanitize_file_name(path: &str) -> String {
    path.replace(|c: char| c.is_invalid_for_path(), "_")
}

pub fn get_track_directory(track: &Track, library_path: &Path) -> PathBuf {
    let mut track_path = PathBuf::from(library_path);

    let artist_folder = if track.album_artists.len() > 0 {
        track.album_artists.join(", ")
    } else {
        (&track.album_artists[0]).to_owned()
    };
    track_path.push(sanitize_file_name(&artist_folder));
    track_path.push(sanitize_file_name(&track.album));
    track_path
}

fn get_track_filename(track: &Track) -> String {
    let file_name = &format!(
        "{}-{:02} {}",
        &track.disc_number, &track.track_number, &track.title
    );
    sanitize_file_name(file_name)
}

fn get_iterative_filename(filename: &str, extension: &str, destination: &Path) -> PathBuf {
    let mut new_path = PathBuf::from(destination);
    let mut counter = 0;
    new_path.push(format!("{}.{}", filename, extension));

    while new_path.exists() {
        counter += 1;
        new_path.pop();
        new_path.push(format!("{} ({}).{}", filename, counter, extension))
    }

    new_path
}

pub fn is_in_hidden_path(file_path: &Path, relative_to: &Path) -> bool {
    get_source(file_path, relative_to).starts_with(".")
}

fn get_source(track_file_path: &Path, relative_to: &Path) -> String {
    match track_file_path.parent().unwrap().strip_prefix(relative_to) {
        Ok(source) if source.to_string_lossy().is_ascii_whitespace() => "None".to_owned(),
        Ok(source) => sanitize_file_name(&source.to_string_lossy())
            .split("_")
            .next()
            .unwrap_or("None")
            .to_owned(),
        Err(_) => "None".to_owned(),
    }
}

fn ensure_not_added(auto_add_path: &Path) -> io::Result<PathBuf> {
    let mut not_added = PathBuf::from(auto_add_path);
    let local: DateTime<Local> = Local::now();
    not_added.push(".notadded");
    not_added.push(local.format("%Y-%m-%d").to_string());
    match fs::create_dir_all(&not_added) {
        Ok(_) => Ok(not_added),
        Err(err) => Err(err),
    }
}

pub fn move_non_track(path: &Path, auto_add_path: &Path) -> Result<()> {
    if let Ok(notadded) = ensure_not_added(auto_add_path) {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let filename = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed file");
        let new_file_name = get_iterative_filename(filename, ext, &notadded);
        if let Err(_) = fs::rename(path, &new_file_name) {
            return Err(Error::UnableToMove(
                new_file_name.to_string_lossy().into_owned(),
            ));
        } else {
            return Ok(());
        }
    }
    Err(Error::UnableToMove("not added folder".to_owned()))
}

fn track_warrants_move(track_as_saved: &Track, track_as_read: &Track) -> bool {
    !(track_as_saved.title == track_as_read.title 
        && track_as_saved.album == track_as_read.album
        && track_as_saved.artist == track_as_read.artist
        && track_as_saved.track_number == track_as_read.track_number
        && track_as_saved.album_artists == track_as_read.album_artists)
}

/// Reconsider the location of a track.
/// If the file is gone or deleted, returns Ok(None).
/// Otherwise, returns a new Track that has a new
/// or same location, depending if its properties have changed.
pub fn reconsider_track(track: &Track, library_path: &Path) -> Result<Option<Track>> {
    let track_file_path = Path::new(&track.file_path);
    if !track_file_path.exists() {
        return Ok(None);
    }

    if let Ok(track_as_read) = Track::new(track_file_path, Some(&track.source)) {
        if !track_warrants_move(track, &track_as_read) {
            return Ok(Some(track_as_read));
        }
        match move_track(&track_as_read, library_path, &track_as_read.source) {
            Ok(track) => {
                //  Cleanup
                if let Some(old_dir) = &track_file_path.parent() {
                    // If the directory is empty, simply remove it.
                    fs::remove_dir(old_dir).unwrap_or(());
                    if let Some(old_dir) = old_dir.parent() {
                        // Cleanup after the artist as well.
                        fs::remove_dir(old_dir).unwrap_or(());
                    }
                }
                Ok(Some(track))
            }
            Err(err) => Err(err),
        }
    } else {
        Ok(None)
    }
}

/// Moves the given track to its proper destination in the library, relative
/// to the Automatically Add to Library path.
pub fn move_new_track(track: &Track, library_path: &Path, auto_add_path: &Path) -> Result<Track> {
    // The original path where the track was found.
    let original_path = Path::new(&track.file_path);

    // The name of the first subfolder from the Automatically Add to Library path
    // and marks it as the source.
    let source = get_source(original_path, auto_add_path);

    move_track(track, library_path, &source)
}

/// Moves a track to its proper position in the library, with the given source.
pub fn move_track(track: &Track, library_path: &Path, source: &str) -> Result<Track> {
    let track_file_path = Path::new(&track.file_path);

    // get the track file extension
    let track_ext = {
        if !track_file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(".")
            .starts_with(".")
        {
            Path::new(&track.file_path)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_owned()
        } else {
            // Handle dotfiles.
            track_file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap()
                .trim_left_matches('.')
                .to_owned()
        }
    };

    // The new filename of the track, from the track metadata.
    let track_file_name = get_track_filename(&track);

    // The new directory of the track in the library, from track metadata
    let track_folder = get_track_directory(&track, &library_path);

    // Ensure the new directory
    if let Err(_) = fs::create_dir_all(&track_folder) {
        return Err(Error::UnableToCreateDirectory(
            track_folder.to_string_lossy().into_owned(),
        ));
    }

    // Make sure not to overwrite any files.
    let new_file_name = get_iterative_filename(&track_file_name, &track_ext, &track_folder);

    // Do the move.
    if let Err(_) = fs::rename(track_file_path, &new_file_name) {
        Err(Error::UnableToMove(
            new_file_name.to_string_lossy().into_owned(),
        ))
    } else {
        Track::new(&new_file_name, Some(&source))
    }
}

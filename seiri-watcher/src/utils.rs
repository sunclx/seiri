use std::io;
use std::path::Path;
use seiri::Bang;
use seiri::database::query_tracks;
use seiri::database::Connection;
use seiri::paths::reconsider_track;
use seiri::config::get_config;

pub fn wait_for_exit(conn: &Connection) {
    let stdin = io::stdin();
    println!("Type 'exit' to exit");
    let folder = get_config().music_folder;
    let library_path = Path::new(&folder);
    let mut input = String::new();
    while let Ok(_) = stdin.read_line(&mut input) {
        if input.trim().eq_ignore_ascii_case("exit") {
            return;
        }
        if input.trim().starts_with("refresh") {
            let file_name: &str = match input.trim().splitn(2, " ").nth(1) {
                Some(query_str) => query_str,
                None => "",
            };
            let track = query_tracks(Bang::FilePath(file_name.to_owned()), conn, None, None).unwrap();
            match track.into_iter().next() {
                Some(track) => {
                    reconsider_track(&track, &library_path).unwrap();
                }
                None => {
                    println!("Some Error")
                }
            };
        }
        if input.trim().starts_with("query") {
            let query_str: &str = match input.trim().splitn(2, " ").nth(1) {
                Some(query_str) => query_str,
                None => "",
            };

            match Bang::new(query_str) {
                Ok(bang) => {
                    println!("{:?}", bang);
                  //  println!("Compiles to... -------------");
                    let tracks = query_tracks(bang, conn, None, None);
                    println!("{:?}", tracks)
                },
                Err(err) => println!("{:?}", err),
            }
        }
        input.clear();
        continue;
    }
}

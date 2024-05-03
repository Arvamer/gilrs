//! This build script takes the gamecontrollerdb.txt from the SDL repo and removes any
//! mappings that aren't for the current platform and removes comments etc.
//!
//! This reduces the binary size fairly significantly compared to including mappings for every
//! platform.
//! Especially Wasm since it doesn't use SDL mappings and binary size is important.

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[cfg(windows)]
const PATH_SEPARATOR: &str = "backslash";

#[cfg(not(windows))]
const PATH_SEPARATOR: &str = "slash";

fn main() {
    println!(r#"cargo:rustc-cfg=path_separator="{}""#, PATH_SEPARATOR);

    let out_dir = env::var("OUT_DIR").unwrap();
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let sdl_platform = "platform:".to_string()
        + match env::var("CARGO_CFG_TARGET_FAMILY").unwrap().as_str() {
            "unix" => match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
                "android" => "Android",
                "macos" => "Mac OS X",
                _ => "Linux",
            },
            "windows" => "Windows",
            "wasm" => "Web",
            _ => "Unknown",
        };

    let sdl_game_controller_db_path: PathBuf =
        PathBuf::from_iter(vec!["SDL_GameControllerDB", "gamecontrollerdb.txt"]);

    // Tell cargo to re-run this script only when SDL's gamecontrollerdb.txt changes.
    println!(
        "cargo:rerun-if-changed={}",
        sdl_game_controller_db_path.to_string_lossy()
    );

    let mut new_file = File::create(Path::new(&out_dir).join("gamecontrollerdb.txt"))
        .expect("failed to create gamecontrollerdb.txt for target");

    let path = Path::new(&cargo_manifest_dir).join(sdl_game_controller_db_path);

    let original_file = File::open(&path).unwrap_or_else(|_| {
        panic!(
            "Could not open gamecontrollerdb.txt {:?}. Did you forget to pull the \
             `SDL_GameControllerDB` submodule?",
            &path
        )
    });
    let original_reader = BufReader::new(original_file);

    original_reader
        .lines()
        .map(|x| match x {
            Ok(x) => x,
            Err(e) => panic!("Failed to read line from gamecontrollerdb.txt: {e}"),
        })
        .filter(|line| {
            line.trim_end()
                .trim_end_matches(',')
                .ends_with(&sdl_platform)
        })
        .for_each(|line| {
            let mut line = line;
            line.push('\n');
            new_file
                .write_all(line.as_bytes())
                .expect("Failed to write line to gamecontrollerdb.txt in OUT_DIR");
        });
}

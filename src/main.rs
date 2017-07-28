extern crate walkdir;
extern crate termion;

use walkdir::{WalkDir, DirEntry};
use std::io::{Write, stdout, stdin};
use termion::input::TermRead;
use termion::event::Key;
use termion::raw::IntoRawMode;

fn is_note(entry: &DirEntry) -> bool {
    entry.file_type().is_file() &&
    entry.file_name()
         .to_str()
         .map(|s| s.ends_with(".txt"))
         .unwrap_or(false)
}

fn main() {
    // Walker experiment.
    let walker = WalkDir::new(".").into_iter();
    for entry in walker.filter_map(|e| e.ok()) {
        if is_note(&entry) {
            println!("{}", entry.path().display());
        }
    }

    // Termion experiment.
    let stdout = stdout();
    let mut stdout = stdout.into_raw_mode().unwrap();
    let stdin = stdin();
    stdout.write_all(b"> ").unwrap();
    stdout.flush().unwrap();
    for key in stdin.keys() {
        match key.unwrap() {
            Key::Ctrl('c') => break,
            Key::Ctrl('d') => break,
            Key::Char(c) => write!(stdout, "{}", c).unwrap(),
            _ => write!(stdout, "*").unwrap(),
        }
        stdout.flush().unwrap();
    }
}

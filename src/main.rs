extern crate walkdir;
extern crate termion;

use walkdir::{WalkDir, DirEntry};
use std::io::{Write, stdout, stdin};
use termion::input::TermRead;
use termion::event::Key;
use termion::raw::IntoRawMode;
use termion::cursor;
use termion::clear;

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
            Key::Char('\n') => break,
            Key::Backspace => {
                write!(stdout, "{}{}",
                       cursor::Left(1),
                       clear::AfterCursor).unwrap();
            },
            Key::Char(c) => write!(stdout, "{}", c).unwrap(),
            _ => {},
        }
        stdout.flush().unwrap();
    }

    // Go to the next line before exiting.
    write!(stdout, "\n\r").unwrap();
    stdout.flush().unwrap();
}

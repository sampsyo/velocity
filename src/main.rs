extern crate walkdir;
extern crate termion;

use walkdir::{WalkDir, DirEntry};
use std::io::{Write, stdout, stdin};
use termion::input::TermRead;
use termion::event::Event;
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
    let mut curstr = String::new();
    let mut curlen = 0;
    for event in stdin.events() {
        match event.unwrap() {
            Event::Key(Key::Ctrl('c')) => break,
            Event::Key(Key::Ctrl('d')) => break,
            Event::Key(Key::Char('\n')) => break,
            Event::Key(Key::Backspace) => {
                match curstr.pop() {
                    Some(_) => {
                        // Move the cursor back.
                        write!(stdout, "{}{}",
                               cursor::Left(1),
                               clear::AfterCursor).unwrap();
                        curlen -= 1;
                    }
                    None => {} // Do nothing.
                }
            }
            Event::Key(Key::Char(c)) => {
                // Add the character to our string.
                curstr.push(c);
                curlen += 1;

                // Show the character.
                write!(stdout, "{}", c).unwrap();

                // Move to the next line.
                write!(stdout, "\n{}{}{}",
                       cursor::Left(1),
                       curlen,
                       cursor::Up(1)).unwrap();
            }
            _ => {},
        }
        stdout.flush().unwrap();
    }

    // Go to the next line before exiting.
    write!(stdout, "\n\r").unwrap();
    stdout.flush().unwrap();
}

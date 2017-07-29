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

const PROMPT: &'static [u8] = b"> ";

fn is_note(entry: &DirEntry) -> bool {
    entry.file_type().is_file() &&
    entry.file_name()
         .to_str()
         .map(|s| s.ends_with(".txt"))
         .unwrap_or(false)
}

// Find the notes matching a term.
// TODO: Keep these matches in memory instead of re-scanning the directory
// every time. Then, incrementally filter in-memory matches when new characters
// are added; perhaps preserve old match lists for when the user hits
// backspace.
// TODO: Do this searching in a separate thread to avoid blocking the UI.
fn find_notes(dir: &str, term: &str) -> Vec<DirEntry> {
    let walker = WalkDir::new(dir).into_iter();
    walker.filter_map(|e| e.ok()).filter(is_note).collect()
}

// Handle an entered search term and display results. Precondition: the
// terminal cursor is at the left-hand edge of the screen, ready to write more
// output. Postcondition: the cursor is returned to that position.
fn run_search(term: &str, stdout: &mut Write) {
    let notes = find_notes(".", &term);
    let mut count = 0;
    for entry in notes {
        if count != 0 {
            write!(stdout, "\n").unwrap();
        }
        write!(stdout, "{}\r", entry.path().display()).unwrap();
        count += 1;
    }

    // Move the cursor back up.
    if count > 1 {
        write!(stdout, "{}", cursor::Up(count - 1)).unwrap();
    }
}

fn interact() {
    let stdout = stdout();
    let mut stdout = stdout.into_raw_mode().unwrap();
    let stdin = stdin();
    stdout.write_all(PROMPT).unwrap();
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
                let posx = (PROMPT.len() + curlen) as u16;
                write!(stdout, "{}\n{}",
                       cursor::Hide,
                       cursor::Left(posx)).unwrap();

                // Run the search.
                run_search(&curstr, &mut stdout);

                // Move *back* to the text entry point.
                write!(stdout, "{}{}{}",
                       cursor::Right(posx),
                       cursor::Up(1),
                       cursor::Show).unwrap();
            }
            _ => {},
        }
        stdout.flush().unwrap();
    }

    // Go to the next line before exiting.
    write!(stdout, "\n\r").unwrap();
    stdout.flush().unwrap();
}

fn main() {
    interact();
}

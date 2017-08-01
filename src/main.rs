extern crate walkdir;
extern crate termion;

use walkdir::{WalkDir, DirEntry};
use std::io::{self, Read, Write, stdout, stdin};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::borrow::Cow;
use termion::input::TermRead;
use termion::event::Event;
use termion::event::Key;
use termion::raw::IntoRawMode;
use termion::cursor;
use termion::clear;

const PROMPT: &'static [u8] = b"> ";
const MAX_MATCHES: usize = 5;

fn is_note(entry: &DirEntry) -> bool {
    entry.file_type().is_file() &&
    entry.file_name()
         .to_str()
         .map(|s| s.ends_with(".txt"))
         .unwrap_or(false)
}

struct Match {
    path: PathBuf,
}

impl Match {
    fn path(&self) -> &Path {
        &self.path
    }

    fn name(&self) -> Cow<str> {
        self.path().file_stem().map(|o| o.to_string_lossy()).
            unwrap_or(Cow::Borrowed("???"))
    }

    // Check whether a note contains a term. If so, return a new Match object.
    // Otherwise, return None.
    fn check(path: &Path, term: &str) -> Result<Option<Match>, io::Error> {
        let mut file = File::open(path)?;

        // TODO: Avoid reading the whole contents into memory at once?
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        if contents.contains(term) {
            Ok(Some(Match { path: path.to_path_buf() }))
        } else {
            Ok(None)
        }
    }
}

// Find the notes matching a term.
// TODO: Keep these matches in memory instead of re-scanning the directory
// every time. Then, incrementally filter in-memory matches when new characters
// are added; perhaps preserve old match lists for when the user hits
// backspace.
// TODO: Do this searching in a separate thread to avoid blocking the UI.
fn find_notes(dir: &str, term: &str) -> Vec<Match> {
    let walker = WalkDir::new(dir).into_iter();
    walker.filter_map(|e| e.ok()).
        filter(is_note).
        filter_map(|e| Match::check(&e.path(), term).unwrap()).
        take(MAX_MATCHES).
        collect()
}

// Handle an entered search term and display results. Precondition: the
// terminal cursor is at the left-hand edge of the screen, ready to write more
// output. Postcondition: the cursor is returned to that position.
// TODO: Show the top match *in* the entry line instead of below, like NV.
fn run_search(term: &str, stdout: &mut Write) {
    let notes = find_notes(".", &term);
    let mut count = 0;
    for m in notes {
        if count != 0 {
            write!(stdout, "\n").unwrap();
        }
        write!(stdout, "{}\r", m.name()).unwrap();
        count += 1;
    }

    // Move the cursor back up.
    if count > 1 {
        write!(stdout, "{}", cursor::Up(count - 1)).unwrap();
    }
}

// TODO Avoid an unpleasant filter by not indiscriminately clearing after
// cursor and instead clearing only the emptied rows.
fn cursor_to_output(stdout: &mut Write) {
    // Move to the next line.
    write!(stdout, "{}\r\n{}",
           cursor::Hide,
           clear::AfterCursor).unwrap();
}

fn cursor_to_input(stdout: &mut Write, curpos: usize) {
    // Move *back* to the text entry point.
    let posx = (PROMPT.len() + curpos) as u16;
    write!(stdout, "{}{}{}",
           cursor::Right(posx),
           cursor::Up(1),
           cursor::Show).unwrap();
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
            // Exit.
            Event::Key(Key::Ctrl('c')) => break,
            Event::Key(Key::Ctrl('d')) => break,

            // TODO: Actually launch $EDITOR.
            Event::Key(Key::Char('\n')) => break,

            Event::Key(Key::Backspace) => {
                match curstr.pop() {
                    Some(_) => {
                        curlen -= 1;

                        // Move the cursor back.
                        write!(stdout, "{}{}",
                               cursor::Left(1),
                               clear::AfterCursor).unwrap();

                        // Run the search.
                        if curlen > 0 {
                            cursor_to_output(&mut stdout);
                            run_search(&curstr, &mut stdout);
                            cursor_to_input(&mut stdout, curlen);
                        }
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

                // Run the search.
                cursor_to_output(&mut stdout);
                run_search(&curstr, &mut stdout);
                cursor_to_input(&mut stdout, curlen);
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

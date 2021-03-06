extern crate walkdir;
extern crate termion;
extern crate textwrap;
extern crate toml;
#[macro_use]
extern crate serde_derive;

use walkdir::{WalkDir, DirEntry};
use std::io::{self, Read, Write, stdout, stdin, Stdout};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::borrow::Cow;
use std::process::Command;
use std::os::unix::process::CommandExt;
use std::env;
use termion::input::TermRead;
use termion::event::{Event, Key};
use termion::raw::{RawTerminal, IntoRawMode};
use termion::{cursor, clear, color, terminal_size};

const PROMPT: &'static [u8] = b"> ";
const MAX_MATCHES: usize = 5;

fn is_note(entry: &DirEntry) -> bool {
    entry.file_type().is_file() &&
    entry.file_name()
         .to_str()
         .map(|s| s.ends_with(".txt"))
         .unwrap_or(false)
}

// Get the human-readable title of a note from its filename.
fn note_name(path: &Path) -> Cow<str> {
    path.file_stem().map(|o| o.to_string_lossy()).
        unwrap_or(Cow::Borrowed("???"))
}

// TODO Use scoring to sort the matches by relevance.
struct Note {
    path: PathBuf,
    contents: String,
    name: String,
}

impl Note {
    fn path(&self) -> &Path {
        &self.path
    }

    // TODO Just show the part that matched.
    fn preview(&self, width: usize) -> String {
        let wrapped = textwrap::fill(&self.contents, width);
        let first = wrapped.lines().next().unwrap();
        String::from(first)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn read(path: &Path) -> Result<Note, io::Error> {
        let mut file = File::open(path)?;

        // TODO: Avoid reading the whole contents into memory at once?
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        Ok(Note {
            path: path.to_path_buf(),
            contents: contents,
            name: String::from(note_name(path)),
        })
    }

    // Check whether a note contains a term.
    // TODO: More efficient case-insensitive matching.
    fn matches(&self, term: &str) -> bool {
        let term_lower = term.to_lowercase();
        self.name.to_lowercase().contains(&term_lower) ||
            self.contents.to_lowercase().contains(&term_lower)
    }
}

// Load all the notes from a given base directory.
// TODO: Do this searching in a separate thread to avoid blocking the UI.
fn load_notes(dir: &Path) -> Vec<Note> {
    let walker = WalkDir::new(dir).into_iter();
    walker.filter_map(|e| e.ok()).
        filter(is_note).
        map(|e| Note::read(&e.path()).unwrap()).
        collect()
}

// Handle an entered search term and display results. Precondition: the
// terminal cursor is at the left-hand edge of the screen, ready to write more
// output. Postcondition: the cursor is returned to that position.
// TODO: Show the top match *in* the entry line instead of below, like NV?
fn show_notes(notes: &Vec<&Note>, stdout: &mut Write) {
    let mut lines = 0;
    for (count, m) in notes.iter().enumerate() {
        // On non-first lines, move down to the next line.
        if count != 0 {
            write!(stdout, "\n").unwrap();
            lines += 1;
        }

        // Show the note's name (and return to the beginning of the line).
        write!(stdout, "{}\r", m.name()).unwrap();

        // Show the preview for the first note.
        if count == 0 {
            let (width, _) = terminal_size().unwrap();
            write!(stdout, "\n{}{}{}\r",
                   color::Fg(color::White),
                   m.preview(width as usize),
                   color::Fg(color::Reset)).unwrap();
            lines += 1;
        }
    }

    // Move the cursor back up.
    if lines >= 1 {
        write!(stdout, "{}", cursor::Up(lines)).unwrap();
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

// The action that the user wants us to take.
enum Action {
    Exit,
    Nothing,
    Edit,
    Search,
}

// Process a single terminal input event. Possibly update the current search
// term state and return the selected action.
// TODO: Arrow keys to move through the result list.
fn handle_event(event: &Event, stdout: &mut Write, curstr: &mut String,
                curlen: &mut usize) -> Action {
    match event {
        // Exit.
        &Event::Key(Key::Ctrl('c')) => return Action::Exit,
        &Event::Key(Key::Ctrl('d')) => return Action::Exit,

        // Launch the user's editor.
        &Event::Key(Key::Char('\n')) => return Action::Edit,

        // Delete a character.
        &Event::Key(Key::Backspace) => {
            match curstr.pop() {
                Some(_) => {
                    *curlen -= 1;

                    // Move the cursor back.
                    write!(stdout, "{}{}",
                           cursor::Left(1),
                           clear::AfterCursor).unwrap();

                    // Search.
                    if *curlen > 0 {
                        return Action::Search;
                    } else {
                        return Action::Nothing;
                    }
                }
                None => {} // Do nothing.
            }
        }

        // Add a character.
        &Event::Key(Key::Char(c)) => {
            // Add the character to our string.
            curstr.push(c);
            *curlen += 1;

            // Show the character.
            write!(stdout, "{}", c).unwrap();

            // Run the search.
            return Action::Search;
        }
        _ => {},
    }
    return Action::Nothing;
}

// Open the user's $EDITOR to edit a given file. This *execs* the editor
// command, so it does not return.
fn edit_file(mut stdout: RawTerminal<Stdout>, path: &Path) {
    // Get the $EDITOR command.
    // TODO Support arguments in the variable.
    // TODO Fallback for when $EDITOR is unset.
    let editor = env!("EDITOR");

    // Preview the command.
    write!(stdout, "\n\r{} {}\n\r",
           editor,
           path.to_string_lossy()).unwrap();
    stdout.flush().unwrap();

    // Drop the raw terminal wrapper to return to normal mode.
    drop(stdout);

    // Run the command.
    // TODO Somehow support non-Unix platforms?
    Command::new(editor)
            .arg(path)
            .exec();
    panic!("editor command failed");
}

// Open the user's $EDITOR for a given note.
// TODO Configurable editor override. For example, this is a nice way to have a
// persistent note window:
// $ mvim --servername note --remote-silent x.txt
fn edit_note(stdout: RawTerminal<Stdout>, note: &Note) {
    edit_file(stdout, note.path());
}

// Create a new note file.
fn create_note(stdout: RawTerminal<Stdout>, base: &Path, name: &str) {
    let path = base.join(name).with_extension("txt");
    edit_file(stdout, &path);
}

fn interact(notes_dir: &Path) {
    let stdout = stdout();
    let mut stdout = stdout.into_raw_mode().unwrap();
    let stdin = stdin();
    stdout.write_all(PROMPT).unwrap();
    stdout.flush().unwrap();

    // The current state of the input. We keep track of the search term itself
    // and its length in typed characters (which we expect to match terminal
    // columns).
    let mut cur_term = String::new();
    let mut cur_term_len: usize = 0;

    // All the notes in the cwd.
    let all_notes = load_notes(&notes_dir);

    // The current set of matched result notes.
    let mut found_notes: Vec<&Note> = Vec::new();

    for event in stdin.events() {
        // Process the event, possibly updating the current text entry.
        let action = handle_event(&event.unwrap(), &mut stdout,
                                  &mut cur_term, &mut cur_term_len);

        // Obey the user's command.
        match action {
            Action::Exit => break,
            Action::Edit => {
                if found_notes.len() > 0 {
                    // We open the first found note.
                    edit_note(stdout, &found_notes[0]);
                } else {
                    // No matching notes: create one.
                    create_note(stdout, &notes_dir, &cur_term);
                }
                return;  // Unreachable: the above calls never return.
            },
            Action::Nothing => {},
            Action::Search => {
                // Run the search to find matching notes.
                found_notes = all_notes.iter()
                    .filter(|n| n.matches(&cur_term))
                    .take(MAX_MATCHES)
                    .collect();

                // If there are results, display them. Otherwise, indicate that
                // we will create a new note.
                cursor_to_output(&mut stdout);
                if found_notes.len() > 0 {
                    show_notes(&found_notes, &mut stdout);
                } else {
                    write!(stdout, "{}*new note*{}\r",
                           color::Fg(color::White),
                           color::Fg(color::Reset)).unwrap();
                }
                cursor_to_input(&mut stdout, cur_term_len);
            },
        }

        stdout.flush().unwrap();
    }

    // Go to the next line before exiting.
    write!(stdout, "\n\r").unwrap();
    stdout.flush().unwrap();
}

#[derive(Deserialize)]
struct Config {
    path: String,
}

// Load the Velocity configuration file.
fn load_config() -> Config {
    let path = env::home_dir().unwrap().join(".config").join("velocity.toml");
    match File::open(&path) {
        Ok(mut f) => {
            let mut contents = String::new();
            f.read_to_string(&mut contents).expect("could not read config");
            toml::from_str(&contents).expect("incomplete config")
        },
        Err(_) => {
            // Use defaults.
            Config {
                path: String::from(".")
            }
        },
    }
}

fn main() {
    let config = load_config();
    let notes_dir = Path::new(&config.path);
    interact(&notes_dir);
}

extern crate walkdir;

use walkdir::{WalkDir, DirEntry};

fn is_note(entry: &DirEntry) -> bool {
    entry.file_type().is_file() &&
    entry.file_name()
         .to_str()
         .map(|s| s.ends_with(".txt"))
         .unwrap_or(false)
}

fn main() {
    let walker = WalkDir::new(".").into_iter();
    for entry in walker.filter_map(|e| e.ok()) {
        if is_note(&entry) {
            println!("{}", entry.path().display());
        }
    }
}

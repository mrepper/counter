use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, ErrorKind, Seek, Write};
use std::path::PathBuf;

use clap::Parser;
use crossterm::event::KeyEvent;
use crossterm::{
    ExecutableCommand, cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{self, ClearType},
};

/// Tally counter with file-backed storage
#[derive(Parser)]
#[command(version, about, long_about = None, max_term_width = 110)]
struct Args {
    /// Path to file where we will store the counter value (will be overwritten)
    #[arg()]
    path: PathBuf,

    /// Starting value (default: 0)
    #[arg()]
    start_value: Option<i64>,

    /// Disable syncing of data to disk on every operation
    #[arg(short, long)]
    no_sync: bool,
}

struct FileCounter {
    file: File,
    count: i64,
    data_sync: bool,
}

fn get_character_choice<'a, T>(
    prompt: &str,
    choice_map: &'a HashMap<KeyEvent, T>,
) -> io::Result<&'a T> {
    loop {
        // Clear line and show prompt
        io::stdout().execute(terminal::Clear(ClearType::CurrentLine))?;
        print!("\r{prompt}");
        io::stdout().flush()?;

        // Read key event, return map value on match
        if let Event::Key(key_event) = event::read()? {
            if let Some(val) = choice_map.get(&key_event) {
                return Ok(val);
            }
        }
    }
}

// Helper function for building KeyEvents for arbitrary KeyCodes
const fn keycode(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::empty())
}

// Helper function for building KeyEvents for single characters
const fn key(c: char) -> KeyEvent {
    keycode(KeyCode::Char(c))
}

fn user_ok_with_overwrite() -> io::Result<bool> {
    let prompt = "File contains non-counter data. Use anyway? (data will be lost!)  [y/n]";

    // Map of input key presses to value we want returned from get_character_choice()
    let choice_map = HashMap::from([
        // Yes
        (key('y'), 'y'),
        (key('Y'), 'y'),
        // No
        (key('n'), 'n'),
        (key('N'), 'n'),
        // Quit
        (key('q'), 'q'),
        (key('Q'), 'q'),
        (
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), // ctrl-c
            'q',
        ),
    ]);

    terminal::enable_raw_mode()?;
    io::stdout().execute(cursor::Hide)?;
    let choice = get_character_choice(prompt, &choice_map)?;
    io::stdout().execute(cursor::Show)?;
    terminal::disable_raw_mode()?;
    println!();
    match choice {
        'y' => Ok(true),
        'n' => Ok(false),
        'q' => std::process::exit(1),
        c => panic!("internal error: unexpected character accepted: '{c}'"),
    }
}

// A counter that persists the count value to a text file
impl FileCounter {
    fn new(path: PathBuf, value: Option<i64>, data_sync: bool) -> Result<Self, io::Error> {
        // Initial count precedence:
        //   1) `value` argument
        //   2) first line of file given by `path` argument
        //   3) 0

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        let mut count: i64 = 0;

        if let Some(value) = value {
            count = value;
        } else {
            let mut reader = BufReader::new(&file);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() {
                if let Ok(value) = line.trim_end().parse::<i64>() {
                    count = value;
                } else if !line.is_empty() && !user_ok_with_overwrite()? {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        "File contained non-counter data",
                    ));
                }
            }
        }

        let mut counter = Self {
            file,
            count,
            data_sync,
        };
        counter.persist()?;
        Ok(counter)
    }

    fn increment(&mut self) -> Result<(), io::Error> {
        match self.count.checked_add(1) {
            None => {
                terminal::disable_raw_mode()?;
                println!("\noverflow!");
                terminal::enable_raw_mode()?;
            }
            Some(val) => self.count = val,
        }

        self.persist()?;
        Ok(())
    }

    fn decrement(&mut self) -> Result<(), io::Error> {
        match self.count.checked_sub(1) {
            None => {
                terminal::disable_raw_mode()?;
                println!("\nunderflow!");
                terminal::enable_raw_mode()?;
            }
            Some(val) => self.count = val,
        }

        self.persist()?;
        Ok(())
    }

    fn persist(&mut self) -> Result<(), io::Error> {
        self.file.seek(io::SeekFrom::Start(0))?;
        self.file.set_len(0)?;
        self.file.write_all(self.count.to_string().as_bytes())?;
        self.file.flush()?;
        if self.data_sync {
            self.file.sync_data()?;
        }
        Ok(())
    }
}

fn main_real() -> Result<(), io::Error> {
    let args = Args::parse();
    let mut counter = FileCounter::new(args.path, args.start_value, !args.no_sync)?;

    // Map of input key presses to value we want returned from get_character_choice()
    let choice_map = HashMap::from([
        // Increment keys
        (key('+'), '+'),
        (key('='), '+'), // '+' without shift
        (key(' '), '+'),
        // Decrement keys
        (key('-'), '-'),
        (key('_'), '-'), // '-' with shift
        (keycode(KeyCode::Backspace), '-'),
        // Quit keys
        (key('q'), 'q'),
        (key('Q'), 'q'),
        (
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), // ctrl-c
            'q',
        ),
    ]);

    terminal::enable_raw_mode()?;
    io::stdout().execute(cursor::Hide)?;
    loop {
        let prompt = format!("Count: {}    [+/-/q]", counter.count);
        let choice = get_character_choice(&prompt, &choice_map)?;
        match choice {
            '+' => counter.increment()?,
            '-' => counter.decrement()?,
            'q' => break,
            c => panic!("internal error: unexpected character accepted: '{c}'"),
        };
    }
    io::stdout().execute(cursor::Show)?;
    terminal::disable_raw_mode()?;
    println!();

    Ok(())
}

fn main() {
    // Show the Display of errors, not Debug
    if let Err(e) = main_real() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

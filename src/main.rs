use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::path::PathBuf;

use clap::Parser;
use crossterm::{
    ExecutableCommand, cursor,
    event::{self, Event, KeyCode},
    terminal::{self, ClearType},
};
use dialoguer::{Confirm, theme::ColorfulTheme};

/// Tally counter
#[derive(Parser)]
#[command(version, about, long_about = None, max_term_width = 110)]
struct Args {
    /// Path to file where we will store the counter value (will be overwritten)
    #[arg()]
    path: PathBuf,

    /// Starting value (default: 0)
    #[arg()]
    start_value: Option<i64>,
}

struct FileCounter {
    path: PathBuf,
    count: i64,
}

// A counter that persists the count in a text file
impl FileCounter {
    fn new(path: PathBuf, value: Option<i64>) -> Result<Self, io::Error> {
        // Initial count precedence:
        //   1) `value` argument
        //   2) first line of file given by `path` argument
        //   3) 0

        let mut count: i64 = 0;

        if let Some(value) = value {
            count = value;
        } else if let Ok(file) = OpenOptions::new().read(true).open(&path) {
            let mut reader = BufReader::new(file);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() {
                if let Ok(value) = line.trim_end().parse::<i64>() {
                    count = value;
                } else {
                    // Prompt the user about using a file that has invalid contents
                    let confirmation = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("File contains invalid data. Use anyway?")
                        .wait_for_newline(true)
                        .interact()
                        .unwrap();
                    if !confirmation {
                        return Err(io::Error::new(
                            ErrorKind::InvalidInput,
                            "Invalid file format",
                        ));
                    }
                }
            }
        }

        let counter = Self { path, count };
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

    fn persist(&self) -> Result<(), io::Error> {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.path)?;

        write!(file, "{}", self.count)?;
        Ok(())
    }
}

fn get_character_choice_crossterm(
    prompt: &str,
    choices: &[char],
    hidden_choices: &[char],
) -> io::Result<char> {
    let choices_str: String = choices
        .iter()
        .map(|&c| format!("{c}"))
        .collect::<Vec<_>>()
        .join("/");

    loop {
        // Clear line and show prompt
        io::stdout().execute(terminal::Clear(ClearType::CurrentLine))?;
        print!("\r{prompt}    [{choices_str}]");
        io::stdout().flush()?;

        // Read key event
        if let Event::Key(key_event) = event::read()? {
            if let KeyCode::Char(ch) = key_event.code {
                if choices.contains(&ch) || hidden_choices.contains(&ch) {
                    return Ok(ch);
                }
            } else if KeyCode::Backspace == key_event.code {
                return Ok('-');
            }
        }
    }
}

fn main_real() -> Result<(), io::Error> {
    let args = Args::parse();
    let mut counter = FileCounter::new(args.path, args.start_value)?;

    terminal::enable_raw_mode()?;
    io::stdout().execute(cursor::Hide)?;
    loop {
        let prompt = format!("{}", counter.count);
        let choice =
            get_character_choice_crossterm(&prompt, &['+', '-', 'q'], &['=', '_', 'Q', ' '])?;
        match choice {
            '+' | '=' | ' ' => counter.increment()?,
            '-' | '_' => counter.decrement()?,
            'q' | 'Q' => break,
            _ => panic!("invalid input"),
        };
    }
    io::stdout().execute(cursor::Show)?;
    terminal::disable_raw_mode()?;
    println!();

    Ok(())
}

fn main() {
    if let Err(e) = main_real() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

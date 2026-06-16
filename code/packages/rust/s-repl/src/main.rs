//! The `s` binary — an interactive prompt for the historical Bell Labs S language.
//!
//! Reads one line at a time, accumulating continuation lines until a statement
//! is complete, then evaluates it and prints any visible result. Type `q()` (or
//! `:quit`, or send EOF with Ctrl-D) to exit.

use coding_adventures_s_repl::{ReplResponse, SRepl};
use std::io::{self, BufRead, Write};

fn main() {
    let mut repl = SRepl::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    println!("S — historical Bell Labs S (v1). Type q() to quit.");

    let mut lines = stdin.lock().lines();
    loop {
        // Show the prompt (`> ` fresh, `+ ` continuing) without a trailing newline.
        print!("{}", repl.prompt());
        stdout.flush().ok();

        match lines.next() {
            Some(Ok(line)) => match repl.feed(&line) {
                ReplResponse::Output(text) => {
                    print!("{text}");
                    stdout.flush().ok();
                }
                ReplResponse::NeedMore => {}
                ReplResponse::Quit => break,
            },
            // EOF (Ctrl-D) or a read error ends the session.
            _ => {
                println!();
                break;
            }
        }
    }
}

//! The `pif-pp` command line: the printer a person reads (VER32 1.1, 3.6).
//!
//! ```text
//!   pif-pp file  <path>     print a PIF file (the prelude, a model) in the printed syntax
//!   pif-pp check <path>     decode, print, parse back and compare: prints `ok` or the reason
//!   pif-pp goal  <cte>      print a goal given as a display CTE
//! ```
//!
//! Exit 0 on success, 1 when the input is refused (the reason on standard error), 2 on a
//! usage error. The output is deterministic: the same input prints the same bytes.

use crate::cli::{read_text, Outcome};
use crate::cte::Dterm;
use crate::digest::Digest;
use crate::filemodel::decode;
use crate::filetext::{parse_file, print_file};
use crate::printcheck::check_goal;

const USAGE: &str = "usage: pif-pp file <path> | pif-pp check <path> | pif-pp goal <display CTE>\n";

fn refuse(why: String) -> Outcome {
    Outcome {
        code: 1,
        stdout: String::new(),
        stderr: format!("pif-pp: {why}\n"),
    }
}

/// Run the printer on `args` (without the program name).
pub fn run(args: &[String]) -> Outcome {
    let [cmd, arg] = args else {
        return Outcome {
            code: 2,
            stdout: String::new(),
            stderr: format!("pif-pp: expected a command and one argument\n{USAGE}"),
        };
    };
    let ok = |s: String| Outcome {
        code: 0,
        stdout: s,
        stderr: String::new(),
    };
    match cmd.as_str() {
        "file" | "check" => {
            let text = match read_text("file", arg) {
                Ok(t) => t,
                Err(e) => return refuse(e.detail()),
            };
            let model = match decode(&text) {
                Ok(m) => m,
                Err(e) => return refuse(format!("cannot decode: {e:?}")),
            };
            let printed = match print_file(&model) {
                Ok(p) => p,
                Err(e) => return refuse(format!("cannot print: {e:?}")),
            };
            // `file` and `check` both run the round trip: nothing is printed that the independent
            // parser does not read back to the same file.
            match parse_file(&printed) {
                Ok(back) if back == model => ok(if cmd == "file" {
                    printed
                } else {
                    "ok\n".to_string()
                }),
                Ok(_) => refuse("parse(print(F)) differs from F".to_string()),
                Err(e) => refuse(format!("the printed text does not parse: {e:?}")),
            }
        }
        "goal" => match Dterm::parse(arg) {
            Ok(g) => match check_goal(&g) {
                // Also the goal digest, so a signer can compare it with the gate's.
                Ok(text) => match g.binding() {
                    Ok(b) => ok(format!(
                        "{text}\ngoal-digest {}\n",
                        Digest::of(b.as_bytes()).hex()
                    )),
                    Err(e) => refuse(format!("{e:?}")),
                },
                Err(e) => refuse(format!("{e:?}")),
            },
            Err(e) => refuse(format!("not a display CTE: {e:?}")),
        },
        _ => Outcome {
            code: 2,
            stdout: String::new(),
            stderr: format!("pif-pp: unknown command {cmd}\n{USAGE}"),
        },
    }
}

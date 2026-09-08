use std::{
    env, fs,
    io::{self, Read},
    process,
};

use spice_netlist_parser::{inspect_netlist_json, run_netlist_json, CLI_ERROR_CODE};

const USAGE: &str = "usage: spice-netlist-parser <inspect|run> --json <deck|- >";

fn read_deck(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut text = String::new();
        io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| error.to_string())?;
        Ok(text)
    } else {
        fs::read_to_string(path).map_err(|error| error.to_string())
    }
}

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 3
        || !matches!(arguments[0].as_str(), "inspect" | "run")
        || arguments[1] != "--json"
    {
        eprintln!("{USAGE}");
        process::exit(2);
    }
    let result = read_deck(&arguments[2]).and_then(|text| match arguments[0].as_str() {
        "inspect" => inspect_netlist_json(&text).map_err(|error| error.to_string()),
        "run" => run_netlist_json(&text).map_err(|error| error.to_string()),
        _ => unreachable!("usage validation accepts only inspect or run"),
    });
    match result {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("{CLI_ERROR_CODE}: {error}");
            process::exit(1);
        }
    }
}

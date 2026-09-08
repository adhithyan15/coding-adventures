use std::{
    env, fs,
    io::{self, Read},
    process,
};

use spice_netlist_parser::run_netlist_json;

const USAGE: &str = "usage: spice-netlist-parser run --json <deck|- >";

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
    if arguments.len() != 3 || arguments[0] != "run" || arguments[1] != "--json" {
        eprintln!("{USAGE}");
        process::exit(2);
    }
    match read_deck(&arguments[2])
        .and_then(|text| run_netlist_json(&text).map_err(|error| error.to_string()))
    {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("SPICE_CLI_ERROR: {error}");
            process::exit(1);
        }
    }
}

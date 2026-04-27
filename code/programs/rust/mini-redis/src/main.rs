use std::env;
use std::path::PathBuf;
use std::process;

use mini_redis::{MiniRedisOptions, MiniRedisServer};

fn main() {
    let options = parse_args(env::args().collect());
    println!("Starting mini-redis on {}:{}", options.host, options.port);

    match MiniRedisServer::new(options.clone()) {
        Ok(server) => {
            if let Some(ref path) = options.aof_path {
                println!("Using AOF persistence at {:?}", path);
            } else {
                println!("Running without persistence.");
            }

            if let Err(err) = server.serve_forever() {
                eprintln!("Server error: {err}");
                process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("Failed to initialize data store: {err}");
            process::exit(1);
        }
    }
}

fn parse_args(args: Vec<String>) -> MiniRedisOptions {
    let mut options = MiniRedisOptions::default();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                if i + 1 < args.len() {
                    options.port = args[i + 1].parse().unwrap_or(6379);
                    i += 1;
                }
            }
            "--host" | "-H" => {
                if i + 1 < args.len() {
                    options.host = args[i + 1].clone();
                    i += 1;
                }
            }
            "--appendonly" | "-a" => {
                if i + 1 < args.len() {
                    options.aof_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--max-connections" => {
                if i + 1 < args.len() {
                    options.max_connections =
                        args[i + 1].parse().unwrap_or(options.max_connections);
                    i += 1;
                }
            }
            "-h" | "--help" => {
                println!(
                    "Usage: mini-redis [--host <ip>] [--port <port>] [--appendonly <path>] [--max-connections <count>]"
                );
                process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                process::exit(1);
            }
        }
        i += 1;
    }

    options
}

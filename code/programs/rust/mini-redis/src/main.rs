use std::env;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;

use in_memory_data_store::DataStorePipeline;
use tcp_server::TcpServer;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let mut port: u16 = 6379;
    let mut aof_path: Option<PathBuf> = None;
    let mut host = "127.0.0.1".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or(6379);
                    i += 1;
                }
            }
            "--host" | "-H" => {
                if i + 1 < args.len() {
                    host = args[i + 1].clone();
                    i += 1;
                }
            }
            "--appendonly" | "-a" => {
                if i + 1 < args.len() {
                    aof_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "-h" | "--help" => {
                println!("Usage: mini-redis [--host <ip>] [--port <port>] [--appendonly <path>]");
                process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                process::exit(1);
            }
        }
        i += 1;
    }

    println!("Starting mini-redis on {}:{}", host, port);
    
    let pipeline_result = DataStorePipeline::new(aof_path.clone());
    match pipeline_result {
        Ok(pipeline) => {
            if let Some(ref path) = aof_path {
                println!("Using AOF persistence at {:?}", path);
            } else {
                println!("Running without persistence.");
            }

            pipeline.start_background_workers();
            let shared_pipeline = Arc::new(pipeline);

            let server = TcpServer::with_handler(host, port, move |conn, data| {
                conn.read_buffer.extend_from_slice(data);
                shared_pipeline.execute(&mut conn.read_buffer, &mut conn.selected_db)
            });

            if let Err(e) = server.serve_forever() {
                eprintln!("Server error: {}", e);
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize pipeline: {}", e);
            process::exit(1);
        }
    }
}

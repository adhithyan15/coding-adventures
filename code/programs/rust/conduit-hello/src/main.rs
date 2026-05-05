use conduit::Server;
use conduit_hello::{build_app, HOST, PORT};

fn main() {
    let app = build_app();

    println!(
        "{} v{}",
        app.setting("app_name").unwrap_or("Conduit Hello (Rust)"),
        app.setting("version").unwrap_or("0.1.0")
    );
    println!("Listening on http://{HOST}:{PORT}");
    println!();
    println!("Routes:");
    for route in app.routes() {
        println!("  {:<6} {}", route.method, route.pattern);
    }
    println!("  GET    /missing  -> custom 404");
    println!();
    println!("Press Ctrl-C to stop.");

    let mut server = match Server::bind(HOST, PORT, app) {
        Ok(server) => server,
        Err(err) => {
            eprintln!("failed to bind Conduit server: {err}");
            std::process::exit(1);
        }
    };

    if let Err(err) = server.serve() {
        eprintln!("Conduit server stopped with error: {err}");
        std::process::exit(1);
    }
}

use venture_browser_macos::{run, run_for_smoke, DEFAULT_START_URL};

fn main() {
    let mut arguments = std::env::args().skip(1);
    let first = arguments.next();
    let (smoke_seconds, start_url) = if first.as_deref() == Some("--smoke-seconds") {
        let seconds = arguments
            .next()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(1.0);
        let url = arguments
            .next()
            .unwrap_or_else(|| DEFAULT_START_URL.to_string());
        (Some(seconds), url)
    } else {
        (None, first.unwrap_or_else(|| DEFAULT_START_URL.to_string()))
    };
    let result = match smoke_seconds {
        Some(seconds) => run_for_smoke(&start_url, seconds),
        None => run(&start_url),
    };
    if let Err(error) = result {
        eprintln!("venture-browser-macos: {error}");
        std::process::exit(1);
    }
}

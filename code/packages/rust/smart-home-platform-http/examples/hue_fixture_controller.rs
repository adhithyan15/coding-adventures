use embeddable_http_server::HttpServerOptions;
use smart_home_platform_http::{
    home_assistant_runtime_web_app, SmartHomePlatformHttpConfig, SmartHomePlatformHttpRuntime,
};
use smart_home_testkit::hue_lighting_runtime;
use std::env;
use std::error::Error;
use std::sync::Arc;
use web_core::WebServer;

fn main() -> Result<(), Box<dyn Error>> {
    let bind_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8123".to_string());
    let runtime = SmartHomePlatformHttpRuntime::new(
        hue_lighting_runtime(),
        SmartHomePlatformHttpConfig::new("Codex Home").with_time_zone("America/Los_Angeles"),
    )
    .with_now_ms(5_000)
    .grant_local_full_access("hue-fixture-controller", 1_000);
    let app = Arc::new(home_assistant_runtime_web_app(runtime));

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    let mut server = WebServer::bind_kqueue(&bind_addr, HttpServerOptions::default(), app)?;

    #[cfg(target_os = "linux")]
    let mut server = WebServer::bind_epoll(&bind_addr, HttpServerOptions::default(), app)?;

    #[cfg(target_os = "windows")]
    let mut server = WebServer::bind_windows(&bind_addr, HttpServerOptions::default(), app)?;

    println!(
        "serving smart-home fixture controller on http://{}",
        server.local_addr()
    );
    server.serve()?;
    Ok(())
}

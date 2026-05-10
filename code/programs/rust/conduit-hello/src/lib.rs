//! Rust `conduit-hello` demo application.

use conduit::{
    escape_json_string, halt, html, html_status, json, json_status, redirect, Application,
    RequestExt,
};

pub const HOST: &str = "127.0.0.1";
pub const PORT: u16 = 3000;

/// Build the demo app without binding a socket.
pub fn build_app() -> Application {
    let mut app = Application::new();
    app.set("app_name", "Conduit Hello (Rust)");
    app.set("version", "0.1.0");

    app.before(|req| {
        if req.path() == "/down" {
            Some(halt(503, "Under maintenance"))
        } else {
            None
        }
    });

    app.after(|req, res| {
        eprintln!("[after] {} {} -> {}", req.method(), req.path(), res.status);
    });

    app.get("/", |_| {
        html("<h1>Hello from Conduit (Rust)!</h1><p>Try <a href='/hello/Adhithya'>/hello/Adhithya</a></p>")
    });

    app.get("/hello/:name", |req| {
        let name = escape_json_string(req.param("name").unwrap_or("world"));
        json(format!(
            r#"{{"message":"Hello {name}!","app":"Conduit Hello (Rust)"}}"#
        ))
    });

    app.post("/echo", |req| {
        let body = req.body_text_lossy();
        if body.trim().is_empty() {
            json("{}")
        } else {
            json(body.as_bytes().to_vec())
        }
    });

    app.get("/redirect", |_| redirect("/", 301));

    app.get("/halt", |_| {
        halt(403, "Forbidden - this route always halts")
    });

    app.get("/down", |_| html("Maintenance mode is off - we're up!"));

    app.get("/error", |_| {
        panic!("Intentional error for demo purposes");
    });

    app.not_found(|req| {
        html_status(
            404,
            format!(
                "<h1>404 Not Found</h1><p>No route matches <code>{}</code></p>",
                req.path()
            ),
        )
    });

    app.on_error(|_req, msg| {
        json_status(
            500,
            format!(
                r#"{{"error":"Internal Server Error","detail":"{}"}}"#,
                escape_json_string(msg)
            ),
        )
    });

    app
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_registers_the_expected_routes() {
        let app = build_app();
        let routes: Vec<_> = app
            .routes()
            .iter()
            .map(|route| (route.method.as_str(), route.pattern.as_str()))
            .collect();

        assert_eq!(
            routes,
            vec![
                ("GET", "/"),
                ("GET", "/hello/:name"),
                ("POST", "/echo"),
                ("GET", "/redirect"),
                ("GET", "/halt"),
                ("GET", "/down"),
                ("GET", "/error"),
            ]
        );
        assert_eq!(app.setting("app_name"), Some("Conduit Hello (Rust)"));
    }
}

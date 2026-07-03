// conduit-hello — a demonstration web application built with the Go Conduit
// framework (WEB14). It shows how to wire routes, before-filters, after-hooks,
// custom error/not-found handlers, and application settings using the
// Sinatra-style DSL. The HTTP engine runs in Rust (web-core, WEB00); Go
// closures are invoked via cgo trampolines.
//
// Run:
//
//	CGO_ENABLED=1 go run . &
//	curl http://127.0.0.1:3000/
//	curl http://127.0.0.1:3000/hello/world
//	curl http://127.0.0.1:3000/api/echo -d "ping" -H "Content-Type: text/plain"
package main

import (
	"encoding/json"
	"fmt"
	"log"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/conduit"
)

// buildApp wires all routes and hooks and returns the configured Application.
// Separating configuration from the serve call makes the app testable without
// actually binding to a port in the main function.
func buildApp() *conduit.Application {
	app := conduit.New()
	app.Set("app_name", "conduit-hello")
	app.Set("version", "0.1.0")

	// ── Before-filter ───────────────────────────────────────────────────────
	//
	// Runs before EVERY route. We use it here to block a maintenance URL and
	// to log the incoming request to stdout. Return nil to continue to the
	// route handler; return a *Response to short-circuit.
	app.Before(func(req *conduit.Request) *conduit.Response {
		fmt.Printf("[%s] %s %s\n", req.RemoteAddr(), req.Method(), req.Path())

		// Simulate a maintenance page for a specific path.
		if req.Path() == "/maintenance" {
			resp := conduit.HTML("<h1>Down for maintenance</h1>", 503)
			return &resp
		}
		return nil // continue to route handler
	})

	// ── After-hook ──────────────────────────────────────────────────────────
	//
	// Runs after EVERY handler and before-filter (even on halt). The hook
	// receives the current response, may mutate it, and must return the
	// (possibly modified) response. Here we stamp a "X-Served-By" header.
	//
	// Important: app.GetSetting reads from the native ConduitApp handle. After
	// Bind(), that handle is consumed and freed by the C side. We pre-capture
	// the setting string before registering the hook so the closure never calls
	// GetSetting from inside a request handler (when the native app is gone).
	appName, _ := app.GetSetting("app_name")
	app.After(func(_ *conduit.Request, resp conduit.Response) conduit.Response {
		resp.Headers = append(resp.Headers, conduit.Header{
			Name:  "x-served-by",
			Value: appName,
		})
		return resp
	})

	// ── Routes ───────────────────────────────────────────────────────────────
	//
	// GET / — HTML home page.
	// Pre-capture settings (the native app handle is consumed after Bind).
	appVersion, _ := app.GetSetting("version")
	app.Get("/", func(req *conduit.Request) conduit.Response {
		name, version := appName, appVersion
		return conduit.HTML(fmt.Sprintf(
			"<h1>%s</h1><p>Version %s — a Conduit demo in Go.</p>"+
				"<ul>"+
				"<li><a href=\"/hello/world\">GET /hello/:name</a></li>"+
				"<li><a href=\"/api/echo\">[POST] /api/echo — echoes the body</a></li>"+
				"<li><a href=\"/api/panic\">[GET] /api/panic — triggers the error handler</a></li>"+
				"</ul>",
			name, version,
		))
	})

	// GET /hello/:name — greets the caller by name using a route parameter.
	// Route parameters are declared with a colon prefix and retrieved via
	// req.Param("name").
	//
	// We marshal via encoding/json (not fmt.Sprintf) so special characters in
	// the name (quotes, backslashes, etc.) cannot break the JSON structure.
	app.Get("/hello/:name", func(req *conduit.Request) conduit.Response {
		name, ok := req.Param("name")
		if !ok || strings.TrimSpace(name) == "" {
			return conduit.JSON(`{"error":"name required"}`, 400)
		}
		b, _ := json.Marshal(map[string]string{"greeting": "Hello, " + name + "!"})
		return conduit.JSON(string(b))
	})

	// POST /api/echo — echoes the request body back with the same Content-Type.
	// Demonstrates reading the request body and content type.
	//
	// We whitelist safe content types: arbitrary Content-Type reflection lets an
	// attacker serve a body with Content-Type: text/html, enabling content-type
	// confusion / reflected XSS if the response is ever rendered in a browser.
	app.Post("/api/echo", func(req *conduit.Request) conduit.Response {
		ct := req.ContentType()
		switch {
		case strings.HasPrefix(ct, "application/json"),
			strings.HasPrefix(ct, "text/plain"),
			strings.HasPrefix(ct, "application/octet-stream"):
			// safe to reflect
		default:
			ct = "text/plain; charset=utf-8"
		}
		return conduit.Respond(200, req.BodyString(),
			conduit.Header{Name: "content-type", Value: ct},
		)
	})

	// GET /api/query — returns a query parameter.
	// Try: /api/query?msg=Hello
	app.Get("/api/query", func(req *conduit.Request) conduit.Response {
		msg, ok := req.Query("msg")
		if !ok {
			return conduit.JSON(`{"error":"pass ?msg=..."}`, 400)
		}
		return conduit.JSON(fmt.Sprintf(`{"msg":"%s"}`, msg))
	})

	// GET /api/panic — intentionally panics to demonstrate the error handler.
	// The trampoline's defer/recover catches the panic and routes it through
	// the OnError handler instead of crashing the server.
	app.Get("/api/panic", func(*conduit.Request) conduit.Response {
		panic("deliberate panic from /api/panic — handled by OnError")
	})

	// GET /api/halt — uses Halt to short-circuit early.
	app.Get("/api/halt", func(*conduit.Request) conduit.Response {
		conduit.Halt(418, "I'm a teapot")
		return conduit.Response{} // unreachable; keeps the type-checker happy
	})

	// ── Custom error and not-found handlers ──────────────────────────────────

	// OnError is called when a handler panics (other than a Halt).
	// req.Error() holds the stringified panic value — log it server-side but
	// never reflect it to the client (information disclosure).
	app.OnError(func(req *conduit.Request) conduit.Response {
		log.Printf("conduit-hello: handler error: %s", req.Error())
		return conduit.JSON(`{"error":"internal server error"}`, 500)
	})

	// NotFound is called when no route matches the request path.
	// Use encoding/json so the path (user-controlled) cannot break the JSON.
	app.NotFound(func(req *conduit.Request) conduit.Response {
		b, _ := json.Marshal(map[string]string{"error": "not found", "path": req.Path()})
		return conduit.JSON(string(b), 404)
	})

	return app
}

func main() {
	app := buildApp()
	server, err := app.Bind("127.0.0.1", 3000)
	if err != nil {
		log.Fatalf("conduit-hello: bind failed: %v", err)
	}
	defer server.Close()

	fmt.Printf("conduit-hello listening on http://127.0.0.1:%d\n", server.LocalPort())
	fmt.Println("Press Ctrl-C to stop.")

	// Serve() blocks until server.Stop() is called (e.g. via Ctrl-C / signal).
	if !server.Serve() {
		log.Fatal("conduit-hello: server stopped with an error")
	}
}

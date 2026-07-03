package conduit_test

import (
	"fmt"
	"io"
	"net/http"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/packages/go/conduit"
)

// ── Response helper unit tests ───────────────────────────────────────────────

func TestHTMLDefaults(t *testing.T) {
	r := conduit.HTML("<h1>Hi</h1>")
	if r.Status != 200 {
		t.Fatalf("status: want 200, got %d", r.Status)
	}
	if string(r.Body) != "<h1>Hi</h1>" {
		t.Fatalf("body: %q", r.Body)
	}
	if r.Headers[0].Name != "content-type" || r.Headers[0].Value != "text/html; charset=utf-8" {
		t.Fatalf("content-type header: %+v", r.Headers)
	}
}

func TestHelperStatusAndTypes(t *testing.T) {
	if conduit.HTML("x", 201).Status != 201 {
		t.Error("html explicit status")
	}
	if conduit.JSON(`{"ok":1}`).Headers[0].Value != "application/json" {
		t.Error("json content-type")
	}
	if conduit.Text("pong").Headers[0].Value != "text/plain; charset=utf-8" {
		t.Error("text content-type")
	}
	r := conduit.Respond(204, "", conduit.Header{Name: "x-y", Value: "z"})
	if r.Status != 204 || r.Headers[0].Name != "x-y" {
		t.Error("respond custom")
	}
}

func TestRedirect(t *testing.T) {
	r, err := conduit.Redirect("/new")
	if err != nil || r.Status != 302 || r.Headers[0].Value != "/new" {
		t.Fatalf("redirect defaults: %+v %v", r, err)
	}
	r2, _ := conduit.Redirect("/old", 301)
	if r2.Status != 301 {
		t.Error("redirect explicit status")
	}
	if _, err := conduit.Redirect("/x\r\nSet-Cookie: evil=1"); err == nil {
		t.Error("redirect must reject CR/LF")
	}
}

// ── Application unit tests ───────────────────────────────────────────────────

func TestSettings(t *testing.T) {
	app := conduit.New()
	defer app.Free()
	app.Set("views", "tmpl")
	if v, ok := app.GetSetting("views"); !ok || v != "tmpl" {
		t.Fatalf("setting round-trip: %q %v", v, ok)
	}
	if _, ok := app.GetSetting("nope"); ok {
		t.Error("missing setting should be absent")
	}
}

func TestChaining(t *testing.T) {
	app := conduit.New()
	defer app.Free()
	if app.Set("a", "1") != app {
		t.Error("Set not chainable")
	}
	if app.Get("/", func(*conduit.Request) conduit.Response { return conduit.Text("x") }) != app {
		t.Error("Get not chainable")
	}
	if app.Before(func(*conduit.Request) *conduit.Response { return nil }) != app {
		t.Error("Before not chainable")
	}
	if app.After(func(_ *conduit.Request, r conduit.Response) conduit.Response { return r }) != app {
		t.Error("After not chainable")
	}
}

func TestBindReturnsPort(t *testing.T) {
	app := conduit.New()
	app.Get("/", func(*conduit.Request) conduit.Response { return conduit.Text("x") })
	server, err := app.Bind("127.0.0.1", 0)
	if err != nil {
		t.Fatalf("bind: %v", err)
	}
	defer server.Close()
	if server.LocalPort() == 0 {
		t.Error("local port should be > 0")
	}
}

// ── End-to-end ───────────────────────────────────────────────────────────────

func TestEndToEnd(t *testing.T) {
	app := conduit.New()
	app.Set("app_name", "conduit-test")

	app.Before(func(req *conduit.Request) *conduit.Response {
		if req.Path() == "/down" {
			conduit.Halt(503, "maintenance")
		}
		return nil
	})

	// Transforming after-hook: stamp a header (full response round-trip).
	app.After(func(_ *conduit.Request, resp conduit.Response) conduit.Response {
		resp.Headers = append(resp.Headers, conduit.Header{Name: "x-served-by", Value: "conduit-go"})
		return resp
	})

	app.Get("/", func(*conduit.Request) conduit.Response { return conduit.HTML("<h1>OK</h1>") })
	app.Get("/hello/:name", func(req *conduit.Request) conduit.Response {
		name, _ := req.Param("name")
		return conduit.JSON(fmt.Sprintf(`{"hi":"%s"}`, name))
	})
	app.Post("/echo", func(req *conduit.Request) conduit.Response {
		ct := req.ContentType()
		if ct == "" {
			ct = "text/plain"
		}
		return conduit.Respond(200, req.BodyString(), conduit.Header{Name: "content-type", Value: ct})
	})
	app.Get("/q", func(req *conduit.Request) conduit.Response {
		v, _ := req.Query("a")
		return conduit.Text("a=" + v)
	})
	app.Get("/boom", func(*conduit.Request) conduit.Response { panic("explode") })
	app.Get("/redir", func(*conduit.Request) conduit.Response {
		r, _ := conduit.Redirect("/", 302)
		return r
	})
	app.NotFound(func(req *conduit.Request) conduit.Response {
		return conduit.Text("no route: "+req.Path(), 404)
	})
	app.OnError(func(*conduit.Request) conduit.Response {
		return conduit.JSON(`{"error":"server"}`, 500)
	})

	server, err := app.Bind("127.0.0.1", 0)
	if err != nil {
		t.Fatalf("bind: %v", err)
	}
	if !server.ServeBackground() {
		t.Fatal("serve background failed")
	}
	defer server.Close()
	defer server.Stop()

	// Watchdog: force the server down after a deadline so nothing hangs.
	watchdog := time.AfterFunc(30*time.Second, func() { server.Stop() })
	defer watchdog.Stop()

	port := server.LocalPort()
	base := fmt.Sprintf("http://127.0.0.1:%d", port)
	client := &http.Client{Timeout: 5 * time.Second}
	waitReady(t, client, base)

	// Disable client-side redirect following so we can assert the 302 itself.
	client.CheckRedirect = func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }

	t.Run("root", func(t *testing.T) {
		st, body, hdr := get(t, client, base+"/")
		assertEq(t, st, 200, "status")
		assertEq(t, body, "<h1>OK</h1>", "body")
		if !strings.Contains(hdr.Get("content-type"), "text/html") {
			t.Errorf("content-type: %q", hdr.Get("content-type"))
		}
		assertEq(t, hdr.Get("x-served-by"), "conduit-go", "after-hook header")
	})

	t.Run("route param", func(t *testing.T) {
		st, body, _ := get(t, client, base+"/hello/world")
		assertEq(t, st, 200, "status")
		assertEq(t, body, `{"hi":"world"}`, "body")
	})

	t.Run("post echo", func(t *testing.T) {
		resp, err := client.Post(base+"/echo", "application/octet-stream", strings.NewReader("ping-pong"))
		if err != nil {
			t.Fatal(err)
		}
		defer resp.Body.Close()
		b, _ := io.ReadAll(resp.Body)
		assertEq(t, resp.StatusCode, 200, "status")
		assertEq(t, string(b), "ping-pong", "body")
		if !strings.Contains(resp.Header.Get("content-type"), "octet-stream") {
			t.Errorf("content-type passthrough: %q", resp.Header.Get("content-type"))
		}
	})

	t.Run("query", func(t *testing.T) {
		_, body, _ := get(t, client, base+"/q?a=42")
		assertEq(t, body, "a=42", "body")
	})

	t.Run("before halt", func(t *testing.T) {
		st, body, _ := get(t, client, base+"/down")
		assertEq(t, st, 503, "status")
		assertEq(t, body, "maintenance", "body")
	})

	t.Run("error handler", func(t *testing.T) {
		st, body, _ := get(t, client, base+"/boom")
		assertEq(t, st, 500, "status")
		assertEq(t, body, `{"error":"server"}`, "body")
	})

	t.Run("not found", func(t *testing.T) {
		st, body, _ := get(t, client, base+"/nope")
		assertEq(t, st, 404, "status")
		assertEq(t, body, "no route: /nope", "body")
	})

	t.Run("redirect", func(t *testing.T) {
		st, _, hdr := get(t, client, base+"/redir")
		assertEq(t, st, 302, "status")
		assertEq(t, hdr.Get("location"), "/", "location")
	})
}

// ── helpers ──────────────────────────────────────────────────────────────────

func waitReady(t *testing.T, client *http.Client, base string) {
	t.Helper()
	for i := 0; i < 100; i++ {
		if resp, err := client.Get(base + "/"); err == nil {
			resp.Body.Close()
			return
		}
		time.Sleep(50 * time.Millisecond)
	}
	t.Fatal("server never became ready")
}

func get(t *testing.T, client *http.Client, url string) (int, string, http.Header) {
	t.Helper()
	resp, err := client.Get(url)
	if err != nil {
		t.Fatalf("GET %s: %v", url, err)
	}
	defer resp.Body.Close()
	b, _ := io.ReadAll(resp.Body)
	return resp.StatusCode, string(b), resp.Header
}

func assertEq[T comparable](t *testing.T, got, want T, what string) {
	t.Helper()
	if got != want {
		t.Errorf("%s: got %v, want %v", what, got, want)
	}
}

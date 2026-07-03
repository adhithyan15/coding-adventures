// Smoke test for conduit-hello. Starts the app on an OS-assigned port, sends
// a handful of real HTTP requests, and asserts the responses. The 30-second
// watchdog ensures the test can never hang CI.
package main

import (
	"fmt"
	"io"
	"net/http"
	"strings"
	"testing"
	"time"
)

// TestSmoke wires the demo app and verifies key routes end-to-end.
func TestSmoke(t *testing.T) {
	app := buildApp()
	server, err := app.Bind("127.0.0.1", 0) // port 0 → OS picks a free port
	if err != nil {
		t.Fatalf("bind: %v", err)
	}
	if !server.ServeBackground() {
		t.Fatal("serve background failed")
	}
	defer server.Stop()
	defer server.Close()

	// Watchdog: stop the server after 30 s so a hung test cannot block CI
	// indefinitely. The defer above also stops it on normal exit, so the
	// double-stop is safe (conduit_server_stop is idempotent).
	watchdog := time.AfterFunc(30*time.Second, func() { server.Stop() })
	defer watchdog.Stop()

	base := fmt.Sprintf("http://127.0.0.1:%d", server.LocalPort())
	client := &http.Client{Timeout: 5 * time.Second}

	// Wait for the server to accept connections.
	waitReady(t, client, base)

	// Disable automatic redirect following so we can inspect 3xx responses.
	client.CheckRedirect = func(*http.Request, []*http.Request) error {
		return http.ErrUseLastResponse
	}

	t.Run("home page", func(t *testing.T) {
		st, body, hdr := get(t, client, base+"/")
		assertEq(t, st, 200, "status")
		if !strings.Contains(body, "conduit-hello") {
			t.Errorf("body: expected 'conduit-hello', got %q", body)
		}
		if !strings.Contains(hdr.Get("content-type"), "text/html") {
			t.Errorf("content-type: %q", hdr.Get("content-type"))
		}
		assertEq(t, hdr.Get("x-served-by"), "conduit-hello", "after-hook header")
	})

	t.Run("route param", func(t *testing.T) {
		_, body, _ := get(t, client, base+"/hello/gopher")
		if !strings.Contains(body, "gopher") {
			t.Errorf("body: expected 'gopher', got %q", body)
		}
	})

	t.Run("echo", func(t *testing.T) {
		resp, err := client.Post(base+"/api/echo", "text/plain", strings.NewReader("pong"))
		if err != nil {
			t.Fatal(err)
		}
		defer resp.Body.Close()
		b, _ := io.ReadAll(resp.Body)
		assertEq(t, string(b), "pong", "echo body")
	})

	t.Run("query", func(t *testing.T) {
		_, body, _ := get(t, client, base+"/api/query?msg=hi")
		if !strings.Contains(body, "hi") {
			t.Errorf("body: expected 'hi', got %q", body)
		}
	})

	t.Run("not found", func(t *testing.T) {
		st, _, _ := get(t, client, base+"/no-such-route")
		assertEq(t, st, 404, "status")
	})

	t.Run("error handler", func(t *testing.T) {
		st, _, _ := get(t, client, base+"/api/panic")
		assertEq(t, st, 500, "status")
	})

	t.Run("halt", func(t *testing.T) {
		st, body, _ := get(t, client, base+"/api/halt")
		assertEq(t, st, 418, "status")
		assertEq(t, body, "I'm a teapot", "body")
	})

	t.Run("maintenance before-filter", func(t *testing.T) {
		st, _, _ := get(t, client, base+"/maintenance")
		assertEq(t, st, 503, "status")
	})
}

// ── helpers ──────────────────────────────────────────────────────────────────

func waitReady(t *testing.T, client *http.Client, base string) {
	t.Helper()
	for i := 0; i < 100; i++ {
		resp, err := client.Get(base + "/")
		if err == nil {
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

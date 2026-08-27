// server.go — Server struct, HTTP route registration, and SSE machinery
//
// The Server struct is the central value that wires together all the moving
// parts of MosaicBook:
//
//   - root        — the directory being scanned for .mosaic files
//   - compilerPath — path (or name) of the mosaic-compile binary
//   - mux         — the HTTP route table
//   - components  — cached list of discovered components (refreshed by watcher)
//   - sseClients  — set of active SSE connections (each gets its own channel)
//   - mu          — guards components and sseClients
//
// # Server-Sent Events (SSE)
//
// SSE is simpler than WebSocket for one-way server→browser push.  Each
// connected browser opens a long-lived GET /events connection.  The server
// keeps a set of per-client channels; when a file change is detected the
// watcher calls s.broadcast(msg) which fans the message out to every channel.
//
// SSE message format:  `data: <json>\n\n`
// The browser reads this as individual MessageEvent objects via EventSource.
//
// # API endpoints
//
//	GET /               → serve static/index.html (the browser shell SPA)
//	GET /api/stories    → JSON list of all discovered components + stories
//	GET /api/backends   → JSON list of backends with availability
//	GET /preview/...    → compiled HTML preview (see preview.go)
//	GET /events         → SSE stream for hot-reload notifications

package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"sync"
)

// Server is the central application state.
type Server struct {
	root         string
	compilerPath string
	mux          *http.ServeMux
	components   []Component
	sseClients   map[chan string]struct{}
	mu           sync.Mutex
}

// newServer constructs a Server, performs an initial component discovery, and
// registers all HTTP handlers on a new ServeMux.
func newServer(root string, compilerPath string) *Server {
	s := &Server{
		root:         root,
		compilerPath: compilerPath,
		mux:          http.NewServeMux(),
		sseClients:   make(map[chan string]struct{}),
	}

	// Initial discovery on startup so /api/stories returns data immediately.
	comps, err := discoverComponents(root)
	if err != nil {
		log.Printf("initial discovery error: %v", err)
	}
	s.components = comps
	log.Printf("discovered %d component(s) in %s", len(comps), root)

	// --- Register HTTP handlers ---
	// All routes use Go 1.22+ method-qualified patterns so the mux does
	// exact-match and prefix routing correctly without a catch-all interfering.

	// GET / — serve the embedded browser shell (exact match).
	s.mux.HandleFunc("GET /{$}", s.handleRoot)

	// API endpoints.
	s.mux.HandleFunc("GET /api/stories", s.handleAPIStories)
	s.mux.HandleFunc("GET /api/backends", s.handleAPIBackends)

	// Preview endpoint — subtree match so component IDs with slashes work.
	s.mux.HandleFunc("GET /preview/", s.handlePreview)

	// Native-backend degradation ("what got dropped") panel — see
	// degradations.go and spec UI19-mosaicbook.md §13.
	s.mux.HandleFunc("GET /api/degradations/", s.handleAPIDegradations)

	// SSE hot-reload stream.
	s.mux.HandleFunc("GET /events", s.handleSSE)

	return s
}

// findComponent looks up a component by ID in the current catalogue, or nil
// if none matches. Shared by handlePreview and handleAPIDegradations so the
// two endpoints can't drift on how a component ID resolves.
func (s *Server) findComponent(id string) *Component {
	s.mu.Lock()
	components := s.components
	s.mu.Unlock()

	for i := range components {
		if components[i].ID == id {
			return &components[i]
		}
	}
	return nil
}

// handleRoot serves the browser shell SPA (static/index.html).
func (s *Server) handleRoot(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path != "/" {
		// 404 for any unknown path — avoid the FileServer fallback.
		http.NotFound(w, r)
		return
	}
	data, err := staticFS.ReadFile("static/index.html")
	if err != nil {
		http.Error(w, "shell not found", http.StatusInternalServerError)
		return
	}
	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	w.Write(data) //nolint:errcheck
}

// handleAPIStories returns the full component catalogue as JSON.
//
// Response schema:
//
//	{ "components": [ { "id": "src/Button", "title": "Button", ... } ] }
func (s *Server) handleAPIStories(w http.ResponseWriter, r *http.Request) {
	// Re-discover on every request so changes made between watcher ticks are
	// visible immediately.  The watcher handles background refresh; this call
	// is an extra safety net and is fast enough for a local dev tool.
	comps, err := discoverComponents(s.root)
	if err != nil {
		http.Error(w, fmt.Sprintf("discovery error: %v", err), http.StatusInternalServerError)
		return
	}

	s.mu.Lock()
	s.components = comps
	s.mu.Unlock()

	type response struct {
		Components []Component `json:"components"`
	}
	writeJSON(w, response{Components: comps})
}

// handleAPIBackends returns the list of backends and their availability.
//
// Tier 1 (browser-native) backends render live in the preview iframe. Tier 3
// (native) backends have no render daemon yet — see UI19-mosaicbook.md §13 —
// so Rendered is false for all five, but degradation Analysis is available
// for all five via GET /api/degradations/{backend}/{component_id}, since
// that needs no daemon or platform runtime.
//
// Response schema:
//
//	{ "backends": [ { "id": "html", "tier": 1, "rendered": true, "analysis": false }, … ] }
func (s *Server) handleAPIBackends(w http.ResponseWriter, r *http.Request) {
	type backend struct {
		ID       string `json:"id"`
		Tier     int    `json:"tier"`
		Rendered bool   `json:"rendered"`
		Analysis bool   `json:"analysis"`
		Reason   string `json:"reason,omitempty"`
	}
	type response struct {
		Backends []backend `json:"backends"`
	}

	const noDaemon = "no render daemon yet — see UI19-mosaicbook.md §13; degradation analysis is available without one"

	resp := response{
		Backends: []backend{
			{ID: "html", Tier: 1, Rendered: true, Analysis: false},
			{ID: "webcomponent", Tier: 1, Rendered: true, Analysis: false},
			{ID: "react", Tier: 1, Rendered: true, Analysis: false},
			{ID: "xaml", Tier: 3, Rendered: false, Analysis: true, Reason: noDaemon},
			{ID: "swiftui", Tier: 3, Rendered: false, Analysis: true, Reason: noDaemon},
			{ID: "qt", Tier: 3, Rendered: false, Analysis: true, Reason: noDaemon},
			{ID: "flutter", Tier: 3, Rendered: false, Analysis: true, Reason: noDaemon},
			{ID: "compose", Tier: 3, Rendered: false, Analysis: true, Reason: noDaemon},
		},
	}
	writeJSON(w, resp)
}

// handleSSE implements the Server-Sent Events endpoint for hot-reload.
//
// The SSE protocol is simple:
//  1. Set Content-Type to text/event-stream.
//  2. Disable buffering (flush after each message).
//  3. Write `data: <json>\n\n` for each event.
//  4. Keep the connection open until the client disconnects.
//
// We register each new client as a channel in s.sseClients.  The watcher
// goroutine calls s.broadcast() which sends to every channel.  When the client
// disconnects (r.Context().Done()), we clean up the channel.
func (s *Server) handleSSE(w http.ResponseWriter, r *http.Request) {
	flusher, ok := w.(http.Flusher)
	if !ok {
		http.Error(w, "streaming not supported", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	w.Header().Set("Access-Control-Allow-Origin", "*")

	// Create a buffered channel so a slow client doesn't block the watcher.
	ch := make(chan string, 16)

	s.mu.Lock()
	s.sseClients[ch] = struct{}{}
	s.mu.Unlock()

	// Clean up on disconnect.
	defer func() {
		s.mu.Lock()
		delete(s.sseClients, ch)
		s.mu.Unlock()
	}()

	// Send a heartbeat comment immediately to confirm the connection is alive.
	// SSE comments (lines starting with ":") are ignored by EventSource.
	fmt.Fprintf(w, ": MosaicBook SSE connected\n\n")
	flusher.Flush()

	// Fan-out loop: forward messages from the channel to the SSE stream.
	for {
		select {
		case msg := <-ch:
			fmt.Fprintf(w, "data: %s\n\n", msg)
			flusher.Flush()
		case <-r.Context().Done():
			// Client disconnected (tab closed, navigation, etc.).
			return
		}
	}
}

// broadcast sends an SSE data payload to all currently connected clients.
// It is safe to call from any goroutine.
func (s *Server) broadcast(msg string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	for ch := range s.sseClients {
		// Non-blocking send: if the client's buffer is full, skip it rather
		// than blocking the watcher.  The client will receive the next reload.
		select {
		case ch <- msg:
		default:
		}
	}
}

// writeJSON marshals v as JSON and writes it to w with the correct
// Content-Type header.  Errors are logged but not returned (the response may
// already be partially written at this point).
func writeJSON(w http.ResponseWriter, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	enc := json.NewEncoder(w)
	enc.SetIndent("", "  ")
	if err := enc.Encode(v); err != nil {
		log.Printf("writeJSON error: %v", err)
	}
}

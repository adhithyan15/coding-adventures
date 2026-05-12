// watcher.go — polling file-system watcher for hot-reload
//
// MosaicBook reloads the browser preview whenever a .mosaic or .stories.json
// file changes.  Rather than depending on OS-specific inotify/kqueue/FSEvents
// APIs (which require third-party packages or CGO), Phase 1 uses a simple
// polling strategy:
//
//   1. Every second, stat all known .mosaic and .stories.json files.
//   2. Compare the modification time (mtime) against the last-seen value.
//   3. If any file changed (or a new file appeared), re-discover all
//      components and broadcast a reload SSE event to all connected clients.
//
// # Trade-offs
//
// Polling at 1-second granularity adds at most 1 second of latency between
// saving a file and seeing the preview update.  This is acceptable for a local
// dev tool.  Polling is also dead-simple, 100% cross-platform, and has no
// external dependencies.
//
// The watcher goroutine runs for the entire lifetime of the server process.
// It terminates when the process exits (no graceful shutdown mechanism needed
// for Phase 1 — this is a local dev tool, not a daemon).

package main

import (
	"log"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// watchFiles is the polling watcher goroutine.  It should be started with
// `go srv.watchFiles()` at server startup.
//
// It polls every second, re-scanning the file tree to detect new files as well
// as modifications to existing ones.  When a change is detected it:
//  1. Re-discovers all components (so /api/stories is immediately up-to-date).
//  2. Broadcasts a reload event to all SSE clients.
func (s *Server) watchFiles() {
	// mtimes maps each tracked file path → last-observed modification time.
	// We include both .mosaic and .stories.json files.
	mtimes := make(map[string]time.Time)

	for {
		// Sleep first so that if the watcher starts before any files are written
		// we don't trigger a spurious reload on startup.
		time.Sleep(1 * time.Second)

		changed := s.pollFiles(mtimes)
		if changed {
			// Re-discover components so the in-memory catalogue is fresh.
			s.mu.Lock()
			comps, err := discoverComponents(s.root)
			if err != nil {
				log.Printf("watcher: re-discover error: %v", err)
			} else {
				s.components = comps
				log.Printf("watcher: detected file change; discovered %d component(s)", len(comps))
			}
			s.mu.Unlock()

			// Broadcast reload to all SSE clients.
			s.broadcast(`{"type":"reload","component_id":"all"}`)
		}
	}
}

// pollFiles walks the root tree looking for .mosaic and .stories.json files,
// comparing their mtimes to the previous snapshot.  It returns true if any
// file was added, modified, or removed since the last call.
//
// mtimes is updated in-place to reflect the current state.
func (s *Server) pollFiles(mtimes map[string]time.Time) bool {
	// Collect the current set of tracked files and their mtimes.
	current := make(map[string]time.Time)

	filepath.Walk(s.root, func(path string, info os.FileInfo, err error) error { //nolint:errcheck
		if err != nil || info == nil || info.IsDir() {
			if info != nil && info.IsDir() && strings.HasPrefix(info.Name(), ".") && path != s.root {
				return filepath.SkipDir
			}
			return nil
		}
		name := info.Name()
		if strings.HasSuffix(name, ".mosaic") || strings.HasSuffix(name, ".stories.json") {
			current[path] = info.ModTime()
		}
		return nil
	})

	changed := false

	// Check for new or modified files.
	for path, mtime := range current {
		if prev, ok := mtimes[path]; !ok || !mtime.Equal(prev) {
			changed = true
			break
		}
	}

	// Check for deleted files.
	if !changed {
		for path := range mtimes {
			if _, ok := current[path]; !ok {
				changed = true
				break
			}
		}
	}

	// Update snapshot.
	for k := range mtimes {
		delete(mtimes, k)
	}
	for k, v := range current {
		mtimes[k] = v
	}

	return changed
}

// Package main is the entry point for mosaicbook-server, a local development
// server that acts as a "Storybook" for Mosaic components. It discovers
// .mosaic files in a project tree, compiles them on-demand to browser-native
// backends (html, webcomponent, react), and serves an interactive preview UI.
//
// Usage:
//
//	mosaicbook-server [--port 7331] [--root .] [--compiler mosaic-compile]
//
// The server runs on localhost only, watches for file changes, and pushes
// hot-reload notifications to connected browsers via Server-Sent Events.
package main

import (
	"flag"
	"fmt"
	"log"
	"net/http"
	"os"
)

func main() {
	// --- Flag definitions ---
	// --port: TCP port the HTTP server binds to. Default 7331 is the MosaicBook
	//         convention (evocative of "Storybook's 6006 + mosaic twist").
	port := flag.Int("port", 7331, "Port to listen on")

	// --root: Directory to scan recursively for .mosaic and .stories.json files.
	//         Defaults to the current working directory so you can run the binary
	//         from your project root without any arguments.
	root := flag.String("root", ".", "Root directory to scan for .mosaic files")

	// --compiler: Path (or name on PATH) of the mosaic-compile binary.
	//             Defaults to "mosaic-compile" so it can be found on PATH after
	//             a normal `go install` of the compiler.
	compiler := flag.String("compiler", "mosaic-compile", "Path to mosaic-compile binary")

	flag.Parse()

	// Resolve the root to an absolute path so the watcher and file paths are
	// unambiguous regardless of where the binary was invoked from.
	absRoot := *root
	if abs, err := os.Getwd(); err == nil && absRoot == "." {
		absRoot = abs
	}

	// Build the central server value.  newServer registers all HTTP routes on
	// its internal mux and initialises the SSE client map.
	srv := newServer(absRoot, *compiler)

	addr := fmt.Sprintf(":%d", *port)
	log.Printf("MosaicBook server running at http://localhost%s", addr)
	log.Printf("Scanning for .mosaic files in: %s", absRoot)
	log.Printf("Using compiler: %s", *compiler)

	// Start the file-system watcher in its own goroutine.  It polls every
	// second and broadcasts a reload event to all connected SSE clients when
	// any .mosaic or .stories.json file changes.
	go srv.watchFiles()

	// Serve HTTP.  log.Fatal terminates on bind error (e.g. port in use).
	log.Fatal(http.ListenAndServe(addr, srv.mux))
}

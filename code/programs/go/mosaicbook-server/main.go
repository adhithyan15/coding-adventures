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
	"context"
	"flag"
	"fmt"
	"log"
	"net/http"
	"os"
	"runtime"
	"time"
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
	check := flag.Bool("check", false, "Compile every explicit story for all browser backends, then exit")
	checkWorkers := flag.Int("check-workers", runtime.NumCPU(), "Maximum concurrent compiler processes in --check mode")
	checkTimeout := flag.Duration("check-timeout", 10*time.Minute, "Overall deadline for --check mode")
	checkDegradations := flag.String("check-degradations", "", "JSON file of issue-linked expected story compile degradations")

	flag.Parse()

	// Resolve the root to an absolute path so the watcher and file paths are
	// unambiguous regardless of where the binary was invoked from.
	absRoot := *root
	if abs, err := os.Getwd(); err == nil && absRoot == "." {
		absRoot = abs
	}

	if *check {
		ctx, cancel := context.WithTimeout(context.Background(), *checkTimeout)
		defer cancel()
		srv := &Server{root: absRoot, compilerPath: *compiler}
		summary, err := srv.checkStories(ctx, *checkWorkers, *checkDegradations)
		if err != nil {
			log.Fatal(err)
		}
		fmt.Printf(
			"MosaicBook story check passed: %d components, %d stories, %d compilations, %d recorded degradations\n",
			summary.Components,
			summary.Stories,
			summary.Compilations,
			summary.RecordedDegradations,
		)
		return
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

	// Serve HTTP, wrapped in requireLocalOrigin (security.go, #13178) so a
	// page from any other origin that reaches this port — including via DNS
	// rebinding, which defeats the localhost bind alone — gets rejected
	// before any route runs.  log.Fatal terminates on bind error (e.g. port
	// in use).
	log.Fatal(http.ListenAndServe(addr, requireLocalOrigin(srv.mux)))
}

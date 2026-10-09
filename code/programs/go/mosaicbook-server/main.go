// Package main is the entry point for mosaicbook-server, a local development
// server that acts as a "Storybook" for Mosaic components. It discovers
// Mosaic files in a project tree, compiles them on-demand to browser-native
// backends or Paint PNG snapshots, and serves an interactive preview UI.
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
	check := flag.Bool("check", false, "Compile every explicit story for all preview backends, then exit")
	checkWorkers := flag.Int("check-workers", runtime.NumCPU(), "Maximum concurrent compiler processes in --check mode")
	checkTimeout := flag.Duration("check-timeout", 10*time.Minute, "Overall deadline for --check mode")
	checkDegradations := flag.String("check-degradations", "", "JSON file of issue-linked expected story compile degradations")

	// --package-search-path: extra directories, in the OS's list syntax, where
	// a package's dependencies are searched after its own siblings. An app
	// under code/programs/mosaic needs code/packages/mosaic here, where its
	// dependencies live (searchpath.go).
	packageSearchPath := flag.String("package-search-path", "", "Extra directories (OS list separator) searched for dependency packages after each package's siblings")

	flag.Parse()

	extraSearchPaths, err := parsePackageSearchPaths(*packageSearchPath)
	if err != nil {
		log.Fatal(err)
	}

	// Resolve the root to an absolute path so the watcher and file paths are
	// unambiguous regardless of where the binary was invoked from.
	absRoot := *root
	if abs, err := os.Getwd(); err == nil && absRoot == "." {
		absRoot = abs
	}

	if *check {
		ctx, cancel := context.WithTimeout(context.Background(), *checkTimeout)
		defer cancel()
		srv := &Server{root: absRoot, compilerPath: *compiler, packageSearchPaths: extraSearchPaths}
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
	srv := newServer(absRoot, *compiler, extraSearchPaths...)

	addr := listenAddress(*port)
	log.Printf("MosaicBook server running at http://localhost:%d", *port)
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

// listenAddress is where the server listens: the loopback interface only.
//
// It used to be ":<port>", which is every interface. The Host-header check
// (requireLocalOrigin) stops a browser page being used against the server
// through DNS rebinding, but not another machine on the network: that
// machine can simply send "Host: localhost". Binding to 127.0.0.1 is what
// keeps the server, and the compiler output it renders, on this machine,
// as the README always said it did.
func listenAddress(port int) string {
	return fmt.Sprintf("127.0.0.1:%d", port)
}

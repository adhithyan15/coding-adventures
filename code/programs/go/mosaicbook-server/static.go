// static.go — embedded static assets via embed.FS
//
// The browser shell (index.html) is embedded directly into the server binary
// using Go's embed package.  This means the single binary is self-contained:
// you don't need to ship a separate static/ directory alongside the executable.
//
// # How embed.FS works
//
// The //go:embed directive tells the Go compiler to include all files matching
// the pattern into the binary at compile time.  The variable staticFS then
// behaves like a read-only file system that you can pass to http.FileServer.
//
// We expose staticFS publicly so server.go can serve it via http.FileServer.

package main

import "embed"

//go:embed static
var staticFS embed.FS

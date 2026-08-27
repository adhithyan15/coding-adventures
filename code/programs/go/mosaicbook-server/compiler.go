// compiler.go — subprocess invocation of the mosaic-compile binary
//
// MosaicBook is a viewer, not a compiler; it delegates compilation to the
// external `mosaic-compile` binary which is part of the main Mosaic toolchain.
// This file wraps `os/exec` calls with helpful error messages and a convenience
// helper that captures compiled output as a string.
//
// # Subprocess model
//
// We invoke one of two forms, depending on how the component is authored --
// see compilerArgs for the details:
//
//	mosaic-compile --backend <backend> --output <outfile> <sourcefile>
//	mosaic-compile --interface <mil> --layout <mll> --style <msl> //	               --package-manifest <toml> --backend <backend> --output <outfile>
//
// The compiler writes its output to <outfile> and exits 0 on success, non-zero
// on failure (with a human-readable error on stderr).
//
// For the preview endpoint we need the compiled output as a Go string so we
// can wrap it in an HTML page and serve it directly.  We achieve this by
// compiling to a temp file, reading the result, and deleting the temp file.

package main

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"os/exec"
)

// maxCompilerOutputBytes is the maximum number of bytes we retain from the
// compiler's combined stdout+stderr.  Bounding this prevents a misbehaving
// compiler from filling server memory with error output.
const maxCompilerOutputBytes = 1 << 20 // 1 MiB

// cappedWriter is an io.Writer that keeps at most limit bytes, silently
// discarding anything beyond that rather than growing without bound.
//
// This is used in place of cmd.CombinedOutput(), which reads a subprocess's
// entire stdout+stderr into memory before any size check can run — a
// misbehaving or hostile subprocess invocation could make the server
// allocate an unbounded amount of memory before a post-hoc truncation ever
// applies (#13179). Capping the buffer as bytes arrive, rather than after
// the fact, is what actually bounds memory use.
//
// Write always reports success for the full input — it never returns fewer
// bytes written than were given, and never errors — so the subprocess's
// stdout/stderr pipe never sees a short write and the subprocess is never
// killed mid-stream merely for exceeding a diagnostic-message size limit.
// This only bounds what the server holds onto, not how much the subprocess
// is allowed to produce or how long it keeps running.
type cappedWriter struct {
	buf     []byte
	limit   int
	written int // total bytes offered, including any discarded past limit
}

func (c *cappedWriter) Write(p []byte) (int, error) {
	c.written += len(p)
	if room := c.limit - len(c.buf); room > 0 {
		if room > len(p) {
			room = len(p)
		}
		c.buf = append(c.buf, p[:room]...)
	}
	return len(p), nil
}

// String returns the captured output, with a truncation marker appended if
// the subprocess wrote more than limit bytes.
func (c *cappedWriter) String() string {
	if c.written > len(c.buf) {
		return string(c.buf) + "\n...(output truncated)"
	}
	return string(c.buf)
}

// compile invokes the external mosaic-compile binary to compile a component to
// the given backend, writing output to outputPath.
//
// Returns a descriptive error if the binary is not found or exits non-zero.
// Compiler output captured for error messages is capped at maxCompilerOutputBytes
// (via cappedWriter) to avoid holding unbounded buffers in memory.
func (s *Server) compile(c Component, backend string, outputPath string) error {
	cmd := exec.Command(s.compilerPath, compilerArgs(c, backend, outputPath)...)

	// Capture combined stdout+stderr so we can surface compiler errors in the
	// preview HTML page rather than just logging them server-side.
	out := &cappedWriter{limit: maxCompilerOutputBytes}
	cmd.Stdout = out
	cmd.Stderr = out
	err := cmd.Run()

	if err != nil {
		// Distinguish "binary not found" from "compilation failed" — the former
		// needs a setup hint, the latter needs the compiler error text.
		if isNotFound(err) {
			return fmt.Errorf(
				"mosaic-compile binary %q not found on PATH; "+
					"build it with: cd code/packages/rust/mosaic-compile && cargo build --release",
				s.compilerPath,
			)
		}
		if out.written > 0 {
			return fmt.Errorf("mosaic-compile failed: %s", out.String())
		}
		return fmt.Errorf("mosaic-compile exited with error: %w", err)
	}
	return nil
}

// compilerArgs builds the mosaic-compile argument list for one component.
//
// There are two invocation forms, and which one applies depends on how the
// component is authored:
//
//   - Legacy single-file: the whole component is one .mosaic file.
//
//     mosaic-compile --backend B --output O source.mosaic
//
//   - Three-file (UI29): interface, layout and style are separate files
//     inside a Mosaic package. This is the form every component in this
//     repo actually uses.
//
//     mosaic-compile --interface X.mil --layout X.mll --style X.msl \
//     --backend B --output O \
//     --package-manifest pkg/mosaic-package.toml
//
// --package-manifest matters more than it looks: it registers the package's
// exported component names so a layout can reference its siblings (Field
// referencing Input, for example). Without it those references fail to
// resolve and the compile errors out.
//
// A three-file component may legitimately have no stylesheet, in which case
// --style is omitted and the backend applies its own defaults.
// Argument injection is the risk to keep in mind here. Go's exec uses no
// shell, so there is no shell injection — but every path in the argv comes
// from a filename in the scanned tree, and a filename can itself look like a
// flag. A top-level file named `--output=pwned.html.mosaic` yields a relative
// source path of exactly that, which mosaic-compile's parser reads as a
// second --output and honours over the real one.
//
// Two defences: three-file component names are constrained to identifiers at
// discovery (validComponentBase), and the legacy positional source is placed
// after a `--` end-of-options separator so it can never be read as a flag.
func compilerArgs(c Component, backend string, outputPath string) []string {
	if !c.isThreeFile() {
		return []string{"--backend", backend, "--output", outputPath, "--", c.SourcePath}
	}

	args := []string{
		"--interface", c.InterfacePath,
		"--layout", c.LayoutPath,
	}
	if c.StylePath != "" {
		args = append(args, "--style", c.StylePath)
	}
	if c.ManifestPath != "" {
		args = append(args, "--package-manifest", c.ManifestPath)
	}
	return append(args, "--backend", backend, "--output", outputPath)
}

// compileToString is a convenience wrapper around compile that returns the
// compiled output as a string.
//
// It creates a temp file in the OS temp directory, compiles into it, reads the
// result, and removes the temp file.  The temp file approach keeps the
// interface identical to the real compiler CLI (which always writes to a file).
func (s *Server) compileToString(c Component, backend string) (string, error) {
	// Create a temp file with an extension appropriate for the backend.
	// The extension doesn't affect correctness but helps debugging when you
	// inspect /tmp during development.
	ext := backendExtension(backend)
	tmp, err := os.CreateTemp("", "mosaicbook-*"+ext)
	if err != nil {
		return "", fmt.Errorf("cannot create temp file: %w", err)
	}
	tmpPath := tmp.Name()
	tmp.Close() // Close before passing path to subprocess (some OS need this).

	// Always remove the temp file, even on error.
	defer os.Remove(tmpPath)

	if err := s.compile(c, backend, tmpPath); err != nil {
		return "", err
	}

	data, err := os.ReadFile(tmpPath)
	if err != nil {
		return "", fmt.Errorf("cannot read compiler output: %w", err)
	}

	return string(data), nil
}

// backendExtension returns the conventional file extension for a compiler
// backend's output.  Used only for the temp-file name — purely cosmetic.
func backendExtension(backend string) string {
	switch backend {
	case "html":
		return ".html"
	case "webcomponent":
		return ".js"
	case "react":
		return ".tsx"
	default:
		return ".out"
	}
}

// isNotFound reports whether err means the compiler binary itself could not
// be found — as opposed to it being found but failing (a syntax error, a
// non-zero exit, etc.) — regardless of whether s.compilerPath is a bare
// name resolved via PATH or an absolute/relative path with a directory
// separator.
//
// Those two forms fail through different layers of os/exec, wrapped
// differently:
//   - A bare name that LookPath can't resolve fails at Command() construction
//     time; the resulting *exec.Error (sentinel exec.ErrNotFound) is stashed
//     in Cmd.Err and returned as-is by Start().
//   - A path containing a separator skips LookPath entirely (Command() only
//     searches PATH when filepath.Base(name) == name) and instead fails when
//     Start() invokes the OS directly via os.StartProcess. That failure comes
//     back as a plain *os.PathError (Op "fork/exec" on Unix) — NOT wrapped in
//     *exec.Error at all — so an earlier version of this function, which only
//     matched by first type-asserting *exec.Error, never saw it: on Windows
//     an absolute nonexistent path apparently still resolves through
//     something PATH-like and DOES produce exec.ErrNotFound, but on Linux it
//     surfaces raw ENOENT instead, uncaught (a real cross-platform gap, only
//     visible on the Linux CI job).
//
// errors.Is walks the full Unwrap() chain regardless of which shape wraps
// it, so checking directly against err (rather than manually unwrapping
// *exec.Error first) covers both: *exec.Error.Unwrap() surfaces
// exec.ErrNotFound, and *os.PathError.Unwrap() surfaces a syscall.Errno that
// satisfies fs.ErrNotExist on every platform Go supports (ENOENT on Unix,
// ERROR_FILE_NOT_FOUND on Windows).
func isNotFound(err error) bool {
	return errors.Is(err, exec.ErrNotFound) || errors.Is(err, fs.ErrNotExist)
}

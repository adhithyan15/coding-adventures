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
	"fmt"
	"os"
	"os/exec"
)

// maxCompilerOutputBytes is the maximum number of bytes we read from the
// compiler's combined stdout+stderr.  Bounding this prevents a misbehaving
// compiler from filling server memory with error output.
const maxCompilerOutputBytes = 1 << 20 // 1 MiB

// compile invokes the external mosaic-compile binary to compile a component to
// the given backend, writing output to outputPath.
//
// Returns a descriptive error if the binary is not found or exits non-zero.
// Compiler output captured for error messages is capped at maxCompilerOutputBytes
// to avoid holding unbounded buffers in memory.
func (s *Server) compile(c Component, backend string, outputPath string) error {
	cmd := exec.Command(s.compilerPath, compilerArgs(c, backend, outputPath)...)

	// Capture combined stdout+stderr so we can surface compiler errors in the
	// preview HTML page rather than just logging them server-side.
	out, err := cmd.CombinedOutput()

	// Cap the captured output to avoid reflecting large blobs into error pages.
	if len(out) > maxCompilerOutputBytes {
		out = append(out[:maxCompilerOutputBytes], []byte("\n...(output truncated)")...)
	}

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
		if len(out) > 0 {
			return fmt.Errorf("mosaic-compile failed: %s", string(out))
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
func compilerArgs(c Component, backend string, outputPath string) []string {
	if !c.isThreeFile() {
		return []string{"--backend", backend, "--output", outputPath, c.SourcePath}
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

// isNotFound reports whether err is the "executable not found" error produced
// by exec.LookPath (wrapped inside *exec.Error).
func isNotFound(err error) bool {
	if execErr, ok := err.(*exec.Error); ok {
		return execErr.Err == exec.ErrNotFound
	}
	return false
}

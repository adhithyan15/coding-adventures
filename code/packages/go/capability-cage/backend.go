// Backend interface for I/O delegation.
//
// The capability cage uses a two-layer design:
//
//  1. The CHECK layer (Manifest.Check) — verifies the operation is declared.
//  2. The DO layer (Backend) — actually performs the operation.
//
// The Backend interface abstracts the "do" layer. This separation enables
// two deployment modes:
//
//   - OpenBackend: delegates directly to Go's standard library (os, net, etc.).
//     This is the default for regular packages and tests.
//
//   - CageBackend (future, D18): sends a JSON-RPC message to the host process,
//     which mediates all I/O on behalf of the sandboxed agent. The cage itself
//     never calls the OS — every operation goes through the host.
//
// In both cases, the capability check runs first. If the manifest does not
// declare the requested operation, the Backend is never called.
//
// The Backend interface deliberately mirrors the secure wrapper functions
// (ReadFile, WriteFile, etc.) so that test code can inject a fake backend
// without touching the real filesystem or network.
package capabilitycage

import (
	"io/fs"
	"net"
	"os"
	"os/exec"
	"time"
)

// Backend abstracts how I/O operations are performed after the capability
// check passes. Implementations must be safe for concurrent use.
type Backend interface {
	// --- Filesystem ---

	// ReadFile reads the named file and returns its contents.
	// Corresponds to os.ReadFile semantics.
	ReadFile(path string) ([]byte, error)

	// WriteFile writes data to the named file, creating or truncating it.
	// Corresponds to os.WriteFile semantics.
	WriteFile(path string, data []byte, perm os.FileMode) error

	// CreateFile creates the named file (empty), failing if it exists.
	CreateFile(path string) error

	// DeleteFile removes the named file.
	DeleteFile(path string) error

	// ListDir returns the names of entries in the named directory.
	ListDir(path string) ([]string, error)

	// --- Network ---

	// Dial opens a network connection to the address on the named network.
	// Corresponds to net.Dial semantics.
	Dial(network, address string) (net.Conn, error)

	// Listen announces on the local network address.
	// Corresponds to net.Listen semantics.
	Listen(network, address string) (net.Listener, error)

	// LookupHost resolves host to a slice of IP addresses.
	// Corresponds to net.LookupHost semantics.
	LookupHost(host string) ([]string, error)

	// --- Process ---

	// Command returns a Cmd that will run name with the given arguments.
	// The caller is responsible for calling Start/Run/Output.
	Command(name string, args ...string) *exec.Cmd

	// Kill sends a signal to a process by PID.
	Kill(pid int, sig os.Signal) error

	// --- Environment ---

	// Getenv retrieves the value of the environment variable named key.
	Getenv(key string) string

	// Setenv sets the value of the environment variable named key.
	Setenv(key, value string) error

	// --- Time ---

	// Now returns the current local time.
	Now() time.Time

	// Sleep pauses execution for at least d.
	Sleep(d time.Duration)

	// --- Stdio ---

	// ReadStdin reads up to len(p) bytes from stdin.
	ReadStdin(p []byte) (int, error)

	// WriteStdout writes p to stdout.
	WriteStdout(p []byte) (int, error)
}

// OpenBackend is the default Backend. It delegates directly to Go's standard
// library with no sandboxing. Use this for production packages and for tests
// that exercise real filesystem/network behavior.
//
// OpenBackend has no state and is safe for concurrent use by multiple goroutines.
type OpenBackend struct{}

// defaultBackend is the package-level Backend used by the secure wrapper
// functions. Swap this in tests via WithBackend if you need a custom backend.
var defaultBackend Backend = &OpenBackend{}

// WithBackend replaces the package-level default backend and returns a function
// that restores the original. This is useful in tests:
//
//	restore := cage.WithBackend(myFakeBackend)
//	defer restore()
func WithBackend(b Backend) func() {
	old := defaultBackend
	defaultBackend = b
	return func() { defaultBackend = old }
}

// --- OpenBackend implementations ---

func (b *OpenBackend) ReadFile(path string) ([]byte, error) {
	return os.ReadFile(path) //nolint:cap
}

func (b *OpenBackend) WriteFile(path string, data []byte, perm os.FileMode) error {
	return os.WriteFile(path, data, perm) //nolint:cap
}

func (b *OpenBackend) CreateFile(path string) error {
	f, err := os.OpenFile(path, os.O_CREATE|os.O_EXCL, 0o644) //nolint:cap
	if err != nil {
		return err
	}
	return f.Close()
}

func (b *OpenBackend) DeleteFile(path string) error {
	return os.Remove(path) //nolint:cap
}

func (b *OpenBackend) ListDir(path string) ([]string, error) {
	entries, err := os.ReadDir(path) //nolint:cap
	if err != nil {
		return nil, err
	}
	names := make([]string, len(entries))
	for i, e := range entries {
		names[i] = e.Name()
	}
	return names, nil
}

func (b *OpenBackend) Dial(network, address string) (net.Conn, error) {
	return net.Dial(network, address) //nolint:cap
}

func (b *OpenBackend) Listen(network, address string) (net.Listener, error) {
	return net.Listen(network, address) //nolint:cap
}

func (b *OpenBackend) LookupHost(host string) ([]string, error) {
	return net.LookupHost(host) //nolint:cap
}

func (b *OpenBackend) Command(name string, args ...string) *exec.Cmd {
	return exec.Command(name, args...) //nolint:cap
}

func (b *OpenBackend) Kill(pid int, sig os.Signal) error {
	p, err := os.FindProcess(pid) //nolint:cap
	if err != nil {
		return err
	}
	return p.Signal(sig) //nolint:cap
}

func (b *OpenBackend) Getenv(key string) string {
	return os.Getenv(key) //nolint:cap
}

func (b *OpenBackend) Setenv(key, value string) error {
	return os.Setenv(key, value) //nolint:cap
}

func (b *OpenBackend) Now() time.Time {
	return time.Now() //nolint:cap
}

func (b *OpenBackend) Sleep(d time.Duration) {
	time.Sleep(d) //nolint:cap
}

func (b *OpenBackend) ReadStdin(p []byte) (int, error) {
	return os.Stdin.Read(p) //nolint:cap
}

func (b *OpenBackend) WriteStdout(p []byte) (int, error) {
	return os.Stdout.Write(p) //nolint:cap
}

// Ensure OpenBackend satisfies the Backend interface at compile time.
var _ Backend = (*OpenBackend)(nil)

// Ensure fs.FileMode is accessible (used by WriteFile signature).
var _ fs.FileMode = os.FileMode(0)

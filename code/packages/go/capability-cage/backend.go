// Backend interface for I/O delegation.
package capabilitycage

import (
	"io/fs"
	"net"
	"os"
	"os/exec"
	"time"
)

// Backend abstracts how I/O operations are performed after the capability check passes.
type Backend interface {
	ReadFile(path string) ([]byte, error)
	WriteFile(path string, data []byte, perm os.FileMode) error
	CreateFile(path string) error
	DeleteFile(path string) error
	ListDir(path string) ([]string, error)
	Dial(network, address string) (net.Conn, error)
	Listen(network, address string) (net.Listener, error)
	LookupHost(host string) ([]string, error)
	Command(name string, args ...string) *exec.Cmd
	Kill(pid int, sig os.Signal) error
	Getenv(key string) string
	Setenv(key, value string) error
	Now() time.Time
	Sleep(d time.Duration)
	ReadStdin(p []byte) (int, error)
	WriteStdout(p []byte) (int, error)
}

// OpenBackend is the default Backend. It delegates directly to Go's standard library.
type OpenBackend struct{}

var defaultBackend Backend = &OpenBackend{}

// WithBackend replaces the package-level default backend and returns a function
// that restores the original.
func WithBackend(b Backend) func() {
	result, _ := StartNew[func()]("capability-cage.WithBackend", func() {},
		func(op *Operation[func()], rf *ResultFactory[func()]) *OperationResult[func()] {
			old := defaultBackend
			defaultBackend = b
			return rf.Generate(true, false, func() { defaultBackend = old })
		}).GetResult()
	return result
}

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

var _ Backend = (*OpenBackend)(nil)
var _ fs.FileMode = os.FileMode(0)

package sandbox

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"path"
	"runtime"
	"strconv"
)

// The two commands of the helper.
const (
	// ExecCommand is the root of the traced tree: `kiln sandbox-exec`.
	ExecCommand = "sandbox-exec"
	// InitCommand is the init of the new PID namespace, which the root
	// re-executes `/proc/self/exe` as.  It is not a user command.
	InitCommand = "sandbox-init"
)

// ExitFailure is the status of a sandbox that failed before the step ran.  125
// is the convention of wrappers (xargs, git bisect run): it cannot be the
// step's own "not found" (127) or "not executable" (126).  A trace of such a
// run has no step exec, and the parser refuses it.
const ExitFailure = 125

// ActionEnv is the variable the build tool's -exec-wrapper sets to the
// package's rel_path.
const ActionEnv = "KILN_ACTION"

// init pins the main goroutine to the process's first thread when this binary
// is a sandbox helper.  The parser's model needs both helper processes to act
// from their thread-group leader: the init must exec the step from it (an exec
// by another thread changes the pid strace prints), and the root must start a
// child from a thread that never exits (Pdeathsig fires when the creating
// THREAD exits).  Calling LockOSThread from an init function is the documented
// way to make main run on the main thread.
func init() {
	if len(os.Args) > 1 && (os.Args[1] == ExecCommand || os.Args[1] == InitCommand) {
		runtime.LockOSThread()
	}
}

// ExecArgs are the parsed arguments of `sandbox-exec`.
type ExecArgs struct {
	Config string
	Argv   []string
}

// ParseExecArgs parses `--config <abs path> -- <argv...>`.
func ParseExecArgs(args []string) (*ExecArgs, error) {
	if len(args) < 4 || args[0] != "--config" || args[2] != "--" {
		return nil, errors.New("usage: kiln sandbox-exec --config <absolute path> -- <argv...>")
	}
	if err := checkPath("config path", args[1]); err != nil {
		return nil, err
	}
	argv := args[3:]
	if err := checkArgv(argv); err != nil {
		return nil, err
	}
	return &ExecArgs{Config: args[1], Argv: argv}, nil
}

// InitArgs are the parsed arguments of `sandbox-init`.
type InitArgs struct {
	Config string
	SHA256 string // hex digest of the configuration the root read
	Action string
	UID    uint32 // the caller's ids, to map
	GID    uint32
	Argv   []string
}

// ParseInitArgs parses
// `--config P --sha256 H --action A --uid N --gid N -- <argv...>`.
func ParseInitArgs(args []string) (*InitArgs, error) {
	bad := errors.New("usage: kiln sandbox-init is internal to sandbox-exec")
	if len(args) < 12 {
		return nil, bad
	}
	var a InitArgs
	for i, want := range []string{"--config", "--sha256", "--action", "--uid", "--gid"} {
		if args[2*i] != want {
			return nil, bad
		}
	}
	if args[10] != "--" {
		return nil, bad
	}
	a.Config, a.SHA256, a.Action = args[1], args[3], args[5]
	u, err1 := strconv.ParseUint(args[7], 10, 32)
	g, err2 := strconv.ParseUint(args[9], 10, 32)
	if err1 != nil || err2 != nil {
		return nil, bad
	}
	a.UID, a.GID = uint32(u), uint32(g)
	if len(a.SHA256) != 64 {
		return nil, bad
	}
	if err := checkPath("config path", a.Config); err != nil {
		return nil, err
	}
	a.Argv = args[11:]
	if err := checkArgv(a.Argv); err != nil {
		return nil, err
	}
	return &a, nil
}

// checkArgv requires a non-empty argv without NUL bytes (they cannot be passed
// to execve and would truncate).
func checkArgv(argv []string) error {
	if len(argv) == 0 {
		return errors.New("sandbox-exec: no command after --")
	}
	for _, a := range argv {
		for i := 0; i < len(a); i++ {
			if a[i] == 0 {
				return errors.New("sandbox-exec: an argument holds a NUL byte")
			}
		}
	}
	return nil
}

// ReadConfig reads the configuration file whole, bounded, and returns its
// bytes and SHA-256.  Both helper processes read it; the root hands the digest
// to the init, so they run the same bytes.
func ReadConfig(p string) ([]byte, string, error) {
	f, err := os.Open(p)
	if err != nil {
		return nil, "", fmt.Errorf("sandbox-exec: %v", err)
	}
	defer f.Close()
	st, err := f.Stat()
	if err != nil || !st.Mode().IsRegular() {
		return nil, "", errors.New("sandbox-exec: the configuration is not a regular file")
	}
	data, err := io.ReadAll(io.LimitReader(f, MaxConfigBytes+1))
	if err != nil {
		return nil, "", fmt.Errorf("sandbox-exec: %v", err)
	}
	if len(data) > MaxConfigBytes {
		return nil, "", errors.New("sandbox-exec: the configuration is too large")
	}
	sum := sha256.Sum256(data)
	return data, hex.EncodeToString(sum[:]), nil
}

// PathLookup lists the candidate paths for a command with no slash, from the
// PATH entry of env.  Only absolute directories count: an empty or relative
// entry would mean the working directory, which is the checkout.
func PathLookup(name string, env []string) []string {
	var pathVar string
	for _, e := range env {
		if len(e) >= 5 && e[:5] == "PATH=" {
			pathVar = e[5:]
		}
	}
	var out []string
	start := 0
	for i := 0; i <= len(pathVar); i++ {
		if i == len(pathVar) || pathVar[i] == ':' {
			if dir := pathVar[start:i]; dir != "" && path.IsAbs(dir) {
				out = append(out, path.Join(dir, name))
			}
			start = i + 1
		}
	}
	return out
}

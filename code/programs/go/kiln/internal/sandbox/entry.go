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

// NoFD is the value of ExecArgs.StdoutFD / StderrFD when the flag is absent:
// the step then inherits the root's own descriptor 1 or 2 as it is.
const NoFD = -1

// ExecArgs are the parsed arguments of `sandbox-exec`.
type ExecArgs struct {
	Config string
	// StdoutFD and StderrFD are the root's descriptors that become the step's
	// 1 and 2 (S2, KILN04 section 4.7), or NoFD.
	StdoutFD int
	StderrFD int
	Argv     []string
}

// maxOutputFD bounds a descriptor number: a plausible one, and small enough
// that the parse cannot overflow.
const maxOutputFD = 1 << 20

// ParseExecArgs parses
// `--config <abs path> [--stdout-fd N --stderr-fd N] -- <argv...>`.
// The two flags come together, in that order, each once; a flag is never
// taken from the step's argv, which starts only after the `--`.
func ParseExecArgs(args []string) (*ExecArgs, error) {
	usage := errors.New("usage: kiln sandbox-exec --config <absolute path> [--stdout-fd N] [--stderr-fd N] -- <argv...>")
	if len(args) < 4 || args[0] != "--config" {
		return nil, usage
	}
	if err := checkPath("config path", args[1]); err != nil {
		return nil, err
	}
	ea := &ExecArgs{Config: args[1], StdoutFD: NoFD, StderrFD: NoFD}
	rest := args[2:]
	if len(rest) >= 2 && rest[0] == "--stdout-fd" {
		n, err := parseOutputFD("--stdout-fd", rest[1])
		if err != nil {
			return nil, err
		}
		ea.StdoutFD, rest = n, rest[2:]
	}
	if len(rest) >= 2 && rest[0] == "--stderr-fd" {
		n, err := parseOutputFD("--stderr-fd", rest[1])
		if err != nil {
			return nil, err
		}
		ea.StderrFD, rest = n, rest[2:]
	}
	if len(rest) < 2 || rest[0] != "--" {
		return nil, usage
	}
	// Both or neither: a recorder that names only one has forgotten the
	// other, and the step's remaining stream would be strace's own, which the
	// recorder means to keep apart (L6).
	if (ea.StdoutFD == NoFD) != (ea.StderrFD == NoFD) {
		return nil, errors.New("sandbox-exec: --stdout-fd and --stderr-fd must be given together or not at all")
	}
	ea.Argv = rest[1:]
	if err := checkArgv(ea.Argv); err != nil {
		return nil, err
	}
	return ea, nil
}

// parseOutputFD reads a descriptor number: decimal digits only (no sign, no
// space, no "0x"), 3 or more.  0 to 2 are the root's own standard streams, which
// are what the step gets by default, so naming one is a mistake, not a choice.
func parseOutputFD(flag, s string) (int, error) {
	if s == "" || len(s) > 7 {
		return 0, fmt.Errorf("sandbox-exec: %s needs a descriptor number, got %q", flag, clip(s))
	}
	n := 0
	for i := 0; i < len(s); i++ {
		if s[i] < '0' || s[i] > '9' {
			return 0, fmt.Errorf("sandbox-exec: %s needs a descriptor number, got %q", flag, clip(s))
		}
		n = n*10 + int(s[i]-'0')
	}
	if n < 3 || n > maxOutputFD {
		return 0, fmt.Errorf("sandbox-exec: %s %d is out of range (3 to %d; 0, 1 and 2 are the root's own streams)", flag, n, maxOutputFD)
	}
	return n, nil
}

// File-type and access-mode bits of a descriptor, spelled out so the check is
// pure and runs on every OS.  The values are Linux's.
const (
	sIFMT   = 0o170000
	sIFIFO  = 0o010000
	oAccmod = 0x3
	oWronly = 0x1
)

// CheckOutputFD decides whether a descriptor may become the step's standard
// output or error, from its file-type bits (st_mode) and its open flags
// (F_GETFL).  It must be a PIPE, open WRITE-ONLY (S2, L4): a descriptor that
// can also read would let the step read back what the recorder's other end
// holds, or hold the read end of a pipe whose reader is the recorder.
//
// Pipes only, because D15 says the step's descriptors 1 and 2 are pipes, and
// because of what the other kinds would give the step: a regular file lets it
// seek back and rewrite what it wrote (and a file in the checkout is one the
// recorder then hashes); a terminal gives it a controlling terminal and its
// ioctls; a socket, a character device (even /dev/null) or a directory is no
// output a reader can attribute.  A pipe is append-only and bounded by its
// reader.  A read end would fail at the first write, and a read-write one
// (a FIFO opened O_RDWR) is refused for the reason above.
func CheckOutputFD(what string, mode uint32, flags int) error {
	if mode&sIFMT != sIFIFO {
		kind := map[uint32]string{
			0o140000: "a socket", 0o120000: "a symbolic link", 0o100000: "a regular file",
			0o060000: "a block device", 0o040000: "a directory", 0o020000: "a character device (a terminal, say)",
		}[mode&sIFMT]
		if kind == "" {
			kind = "not a pipe"
		}
		return fmt.Errorf("sandbox-exec: %s is %s; the step's output must be a pipe", what, kind)
	}
	if flags&oAccmod != oWronly {
		return fmt.Errorf("sandbox-exec: %s is not open write-only", what)
	}
	return nil
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

//go:build !(linux && amd64)

package sandbox

import (
	"fmt"
	"io"
	"runtime"
)

// Supported reports whether this platform can run the sandbox.
func Supported() bool { return false }

func unsupported(stderr io.Writer, cmd string) int {
	fmt.Fprintf(stderr, "kiln %s: the sandbox needs linux/amd64 (this is %s/%s); traced recording is Linux x64 only\n",
		cmd, runtime.GOOS, runtime.GOARCH)
	return ExitFailure
}

// RunExec refuses: there is no sandbox on this platform.
func RunExec(_ []string, stderr io.Writer) int { return unsupported(stderr, ExecCommand) }

// RunInit refuses: there is no sandbox on this platform.
func RunInit(_ []string, stderr io.Writer) int { return unsupported(stderr, InitCommand) }

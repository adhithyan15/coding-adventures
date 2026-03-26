// =========================================================================
// uname — Print System Information
// =========================================================================
//
// The `uname` utility prints information about the operating system and
// hardware. It's one of the most fundamental diagnostic tools on Unix.
//
// # What information does uname provide?
//
// Unix systems have several identity attributes:
//
//   Attribute        Flag   Example (macOS)         Example (Linux)
//   ──────────────   ────   ──────────────────────  ──────────────────────
//   Kernel name      -s     Darwin                  Linux
//   Hostname         -n     myMac.local             web-server-01
//   Kernel release   -r     23.1.0                  6.1.0-9-amd64
//   Kernel version   -v     Darwin Kernel Version   #1 SMP PREEMPT_DYNAMIC
//   Machine          -m     arm64                   x86_64
//   Processor        -p     arm                     x86_64
//   Hardware platf.  -i     arm64                   x86_64
//   Operating sys.   -o     Darwin                  GNU/Linux
//
// # The -a flag
//
// The -a flag prints ALL fields in order:
//   kernel_name nodename kernel_release kernel_version machine processor
//   hardware_platform operating_system
//
// If no flags are given, only the kernel name is printed (same as -s).
//
// # Architecture
//
//   uname.json (spec)            uname_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -a,-s,-n  │       │ gather system info via Go runtime│
//   │ -r,-v,-m,-p,-i,-o│──────>│ select fields based on flags     │
//   │ no arguments     │       │ print space-separated            │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"
	"runtime"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// UnameInfo — holds all the system information fields
// =========================================================================
//
// Each field corresponds to one of uname's flags. We gather all of them
// up front, then select which ones to display based on the flags.

type UnameInfo struct {
	KernelName       string // -s: e.g., "Darwin" or "Linux"
	Nodename         string // -n: the network hostname
	KernelRelease    string // -r: kernel version number
	KernelVersion    string // -v: kernel build info
	Machine          string // -m: hardware architecture (e.g., "arm64")
	Processor        string // -p: processor type
	HardwarePlatform string // -i: hardware platform
	OperatingSystem  string // -o: OS name
}

// =========================================================================
// getSystemInfo — gather system information using Go's runtime/os packages
// =========================================================================
//
// Go doesn't expose all uname(2) fields directly, so we use a mix of:
//   - runtime.GOOS     → kernel name and OS
//   - runtime.GOARCH   → machine and processor
//   - os.Hostname()    → nodename
//
// For kernel release and version, we'd need syscall.Uname on Linux or
// exec("uname -r") on macOS. For portability, we use reasonable defaults.

func getSystemInfo() UnameInfo {
	// Map Go's GOOS to traditional kernel names.
	kernelName := mapKernelName(runtime.GOOS)

	// Get the hostname.
	hostname, err := os.Hostname()
	if err != nil {
		hostname = "unknown"
	}

	// Map Go's GOARCH to traditional machine names.
	machine := mapMachineName(runtime.GOARCH)

	return UnameInfo{
		KernelName:       kernelName,
		Nodename:         hostname,
		KernelRelease:    "unknown", // Would need syscall.Uname for real value
		KernelVersion:    "unknown", // Would need syscall.Uname for real value
		Machine:          machine,
		Processor:        machine,          // Often same as machine
		HardwarePlatform: machine,          // Often same as machine
		OperatingSystem:  mapOSName(runtime.GOOS),
	}
}

// =========================================================================
// mapKernelName — convert Go's GOOS to a traditional kernel name
// =========================================================================
//
// Go uses lowercase names like "darwin", "linux", "windows".
// Traditional uname uses capitalized names like "Darwin", "Linux".

func mapKernelName(goos string) string {
	switch goos {
	case "darwin":
		return "Darwin"
	case "linux":
		return "Linux"
	case "windows":
		return "Windows_NT"
	case "freebsd":
		return "FreeBSD"
	case "openbsd":
		return "OpenBSD"
	case "netbsd":
		return "NetBSD"
	default:
		// Capitalize the first letter as a reasonable default.
		if len(goos) > 0 {
			return strings.ToUpper(goos[:1]) + goos[1:]
		}
		return "Unknown"
	}
}

// =========================================================================
// mapMachineName — convert Go's GOARCH to a traditional machine name
// =========================================================================
//
// Go uses names like "amd64", "arm64", "386".
// Traditional uname uses names like "x86_64", "aarch64", "i686".

func mapMachineName(goarch string) string {
	switch goarch {
	case "amd64":
		return "x86_64"
	case "arm64":
		return "aarch64"
	case "386":
		return "i686"
	case "arm":
		return "armv7l"
	default:
		return goarch
	}
}

// =========================================================================
// mapOSName — convert Go's GOOS to a traditional OS name
// =========================================================================

func mapOSName(goos string) string {
	switch goos {
	case "linux":
		return "GNU/Linux"
	case "darwin":
		return "Darwin"
	case "windows":
		return "Windows"
	case "freebsd":
		return "FreeBSD"
	default:
		return mapKernelName(goos)
	}
}

// =========================================================================
// formatUname — select and format the fields based on flags
// =========================================================================
//
// If no flags are given, default to showing just the kernel name (-s).
// If -a is given, show all fields in the canonical order.

func formatUname(info UnameInfo, showAll, showKernel, showNode, showRelease,
	showVersion, showMachine, showProcessor, showPlatform, showOS bool) string {

	// If -a is set, enable all flags.
	if showAll {
		showKernel = true
		showNode = true
		showRelease = true
		showVersion = true
		showMachine = true
		showProcessor = true
		showPlatform = true
		showOS = true
	}

	// If no flags are set at all, default to kernel name.
	if !showKernel && !showNode && !showRelease && !showVersion &&
		!showMachine && !showProcessor && !showPlatform && !showOS {
		showKernel = true
	}

	// Collect selected fields in canonical order.
	var parts []string
	if showKernel {
		parts = append(parts, info.KernelName)
	}
	if showNode {
		parts = append(parts, info.Nodename)
	}
	if showRelease {
		parts = append(parts, info.KernelRelease)
	}
	if showVersion {
		parts = append(parts, info.KernelVersion)
	}
	if showMachine {
		parts = append(parts, info.Machine)
	}
	if showProcessor {
		parts = append(parts, info.Processor)
	}
	if showPlatform {
		parts = append(parts, info.HardwarePlatform)
	}
	if showOS {
		parts = append(parts, info.OperatingSystem)
	}

	return strings.Join(parts, " ")
}

// =========================================================================
// runUname — the testable core of the uname tool
// =========================================================================

func runUname(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "uname: %s\n", err)
		return 1
	}

	// Step 2: Parse the arguments.
	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintf(stderr, "%s\n", err)
		return 1
	}

	// Step 3: Handle the result.
	switch r := result.(type) {

	case *clibuilder.HelpResult:
		fmt.Fprintln(stdout, r.Text)
		return 0

	case *clibuilder.VersionResult:
		fmt.Fprintln(stdout, r.Version)
		return 0

	case *clibuilder.ParseResult:
		info := getSystemInfo()
		output := formatUname(info,
			getBool(r.Flags, "all"),
			getBool(r.Flags, "kernel_name"),
			getBool(r.Flags, "nodename"),
			getBool(r.Flags, "kernel_release"),
			getBool(r.Flags, "kernel_version"),
			getBool(r.Flags, "machine"),
			getBool(r.Flags, "processor"),
			getBool(r.Flags, "hardware_platform"),
			getBool(r.Flags, "operating_system"),
		)
		fmt.Fprintln(stdout, output)
		return 0

	default:
		fmt.Fprintf(stderr, "uname: unexpected result type: %T\n", result)
		return 1
	}
}

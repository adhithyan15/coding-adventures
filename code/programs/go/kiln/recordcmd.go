package main

// `kiln record` (KILN04 stage 4, section 4.8, decisions P1 and P2)
// ================================================================
//
//	kiln record prepare --root <abs> --dir <abs> --build-tool <abs>
//	                    --shard-index I --shard-count N --job-start <unix>
//	                    [--strace <abs>]
//	kiln record step    --dir <abs> -- sh -c <line>                  (PR 2b-2a)
//	kiln record collect --root <abs> --dir <abs> --results <abs> --out <abs>
//	                                                                 (PR 2b-2b)
//
// `prepare` is the part of a recording shard that runs before any BUILD (see
// internal/record/prepare.go). `step` is the build tool's -exec-wrapper: it
// supervises one BUILD line under strace (internal/record/step.go). `collect`
// is the post-build pass; it belongs to the next PR and exists here only as a
// stub that refuses, so the command surface a workflow is written against is
// fixed now, and a workflow that wires the build to a recorder that does not
// collect yet fails loudly instead of recording nothing.
//
// Refusals come before any work and write nothing:
//
//	exit 2   a usage error (a missing flag, a relative path)
//	exit 3   not a trusted GitHub run, or not linux/amd64
//	exit 1   anything `prepare` found wrong, or a stub
//
// `step` is different, because its exit status is the step's: it exits with the
// traced root's status, or 125 (record.ExitRefused) when it refused to run the
// step or could not account for it, 2 for a usage error and 3 off linux/amd64.
//
// Platform and trust are hooks (package variables) so the tests can exercise the
// whole command on a developer's machine without being a GitHub runner.

import (
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/host"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
)

// recordPlatformOK says whether this process may record. The recorder needs
// user namespaces, strace and the Linux system-call table, and the weak key
// names the architecture, so only linux/amd64 qualifies.
var recordPlatformOK = func() bool { return runtime.GOOS == "linux" && runtime.GOARCH == "amd64" }

// recordHost detects where Kiln runs.
var recordHost = func() host.Host { return host.Detect(os.Getenv, os.Environ()) }

// recordSelf is the canonical path of the running binary: the executable's path
// with every symlink resolved, which is the spelling sandbox.CheckHost demands.
var recordSelf = func() (string, error) {
	exe, err := os.Executable()
	if err != nil {
		return "", err
	}
	return filepath.EvalSymlinks(exe)
}

// recordPrepare is record.Prepare; a hook so a test of the command's refusals
// can prove Prepare was never reached.
var recordPrepare = record.Prepare

// recordStep is record.RunStep; a hook so a test of the command can prove a
// refusal never reached it.
var recordStep = record.RunStep

const notYet = "not implemented (KILN04 stage 4 PR 2b-2b)"

// runRecord dispatches `kiln record <subcommand>`.
func runRecord(args []string, stdout, stderr io.Writer) int {
	if len(args) == 0 {
		fmt.Fprintln(stderr, "usage: kiln record <prepare|step|collect> ...")
		return 2
	}
	switch args[0] {
	case "prepare":
		return runRecordPrepare(args[1:], stdout, stderr)
	case "step":
		return runRecordStep(args[1:], stdout, stderr)
	case "collect":
		// The stub refuses everything, flags included: there is nothing to run.
		fmt.Fprintf(stderr, "kiln: record %s: %s\n", args[0], notYet)
		return 1
	}
	fmt.Fprintf(stderr, "kiln: unknown record subcommand %q\n", args[0])
	return 2
}

func runRecordPrepare(args []string, stdout, stderr io.Writer) int {
	fs := flag.NewFlagSet("record prepare", flag.ContinueOnError)
	fs.SetOutput(stderr)
	root := fs.String("root", "", "the checkout (absolute)")
	dir := fs.String("dir", "", "the record directory, absent or empty (absolute)")
	buildTool := fs.String("build-tool", "", "the build-tool binary, outside the checkout (absolute)")
	strace := fs.String("strace", "/usr/bin/strace", "the strace binary (absolute)")
	gitBin := fs.String("git", "/usr/bin/git", "the git binary every host-side Git call uses (absolute; never looked up on PATH)")
	index := fs.Int("shard-index", -1, "this shard's index")
	count := fs.Int("shard-count", 0, "the number of shards")
	jobStart := fs.Int64("job-start", -1, "Unix time the job's first step stamped")
	if err := fs.Parse(args); err != nil {
		return 2
	}
	usage := "usage: kiln record prepare --root <abs> --dir <abs> --build-tool <abs> --shard-index I --shard-count N --job-start <unix seconds> [--strace <abs>] [--git <abs>]"
	if fs.NArg() != 0 || *root == "" || *dir == "" || *buildTool == "" || *index < 0 || *count < 1 || *jobStart < 0 {
		fmt.Fprintln(stderr, usage)
		return 2
	}
	for _, p := range []struct{ name, v string }{{"--root", *root}, {"--dir", *dir}, {"--build-tool", *buildTool}, {"--strace", *strace}, {"--git", *gitBin}} {
		if !filepath.IsAbs(p.v) {
			fmt.Fprintf(stderr, "kiln: %s %q is not an absolute path\n", p.name, p.v)
			return 2
		}
	}
	// Platform and trust, before anything is read or written.
	if !recordPlatformOK() {
		fmt.Fprintf(stderr, "kiln: record prepare runs only on linux/amd64 (this is %s/%s); nothing was written\n", runtime.GOOS, runtime.GOARCH)
		return 3
	}
	h := recordHost()
	gh, ok := h.(*host.GitHub)
	if !ok || !gh.Trusted() {
		fmt.Fprintln(stderr, "kiln: record prepare needs a trusted GitHub run (default branch; push, schedule or workflow_dispatch); nothing was written")
		return 3
	}
	if strings.Contains(h.ImageID(), "unknown") {
		fmt.Fprintf(stderr, "kiln: record prepare needs a runner image id, not %q; nothing was written\n", h.ImageID())
		return 3
	}
	self, err := recordSelf()
	if err != nil {
		fmt.Fprintf(stderr, "kiln: cannot locate this binary: %v\n", err)
		return 1
	}
	res, err := recordPrepare(record.Options{
		Root: *root, Dir: *dir, BuildTool: *buildTool, KilnExe: self, Strace: *strace, Git: *gitBin,
		ShardIndex: *index, ShardCount: *count, JobStart: *jobStart,
		Image: h.ImageID(), Getenv: os.Getenv, Environ: os.Environ(),
	})
	if err != nil {
		fmt.Fprintf(stderr, "kiln: record prepare: %v\n", err)
		return 1
	}
	// Nothing printed here is a value from the environment.
	fmt.Fprintf(stdout, "ok: shard %d of %d prepared: binding %s, %d actions (%d without a weak key); the build tool's -exec-wrapper is in %s\n",
		res.Binding.ShardIndex, res.Binding.ShardCount, res.BindingID, res.Actions, res.Refused,
		filepath.Join(res.PreDir, "exec-wrapper.json"))
	return 0
}

// runRecordStep is `kiln record step --dir <abs> -- sh -c <line>`.
//
// The step's output goes to stdout and stderr, which in the real process are its
// own (so the build log shows it); the tests hand in buffers.
func runRecordStep(args []string, stdout, stderr io.Writer) int {
	fs := flag.NewFlagSet("record step", flag.ContinueOnError)
	fs.SetOutput(stderr)
	dir := fs.String("dir", "", "the record directory (absolute)")
	if err := fs.Parse(args); err != nil {
		return 2
	}
	if *dir == "" || !filepath.IsAbs(*dir) || fs.NArg() == 0 {
		fmt.Fprintln(stderr, "usage: kiln record step --dir <abs> -- sh -c <line>")
		return 2
	}
	if !recordPlatformOK() {
		fmt.Fprintf(stderr, "kiln: record step runs only on linux/amd64 (this is %s/%s); the step was not run\n", runtime.GOOS, runtime.GOARCH)
		return 3
	}
	self, err := recordSelf()
	if err != nil {
		fmt.Fprintf(stderr, "kiln: record step: cannot locate this binary: %v\n", err)
		return record.ExitRefused
	}
	cwd, err := os.Getwd()
	if err != nil {
		fmt.Fprintf(stderr, "kiln: record step: %v\n", err)
		return record.ExitRefused
	}
	res, err := recordStep(record.StepOptions{
		Dir: *dir, Argv: fs.Args(), Environ: os.Environ(), Cwd: cwd, KilnExe: self,
		Stdout: stdout, Stderr: stderr,
	})
	if err != nil {
		// Nothing printed here is a value from the environment.
		fmt.Fprintf(stderr, "kiln: record step: %v\n", err)
		return record.ExitRefused
	}
	return res.ExitCode
}

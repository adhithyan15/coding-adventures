package main

// `kiln record` (KILN04 stage 4, section 4.8, decisions P1 and P2)
// ================================================================
//
//	kiln record prepare --root <abs> --dir <abs> --build-tool <abs>
//	                    --shard-index I --shard-count N --job-start <unix>
//	                    [--strace <abs>]
//	kiln record step    --dir <abs> -- sh -c <line>                  (PR 2b-2a)
//	kiln record collect --root <abs> --dir <abs> --results <abs> --out <abs>
//	                                                                 (PR 2b-2b-ii)
//
// `prepare` is the part of a recording shard that runs before any BUILD (see
// internal/record/prepare.go). `step` is the build tool's -exec-wrapper: it
// supervises one BUILD line under strace (internal/record/step.go). `collect`
// is the post-build pass (internal/record/collect.go): it assembles the shard's
// `records.jsonl` from the sealed `pre/`, the traces and the build tool's results.
//
// Refusals come before any work and write nothing:
//
//	exit 2   a usage error (a missing flag, a relative path)
//	exit 3   not a trusted GitHub run, or not linux/amd64
//	exit 1   anything `prepare` or `collect` found wrong (`collect` refuses, and
//	         writes nothing, when it cannot say what the build did; a shard that
//	         is VOID, whose records the merge will discard, is still written and
//	         `collect` exits 0: the verdict is the merge's, and the report names it)
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

// recordArmParentDeath makes the supervisor die with the build tool (Linux).
var recordArmParentDeath = armParentDeath

// recordCollect is record.Collect; a hook so a test of the command can prove a
// refusal never reached it.
var recordCollect = record.Collect

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
	case record.ChildCommand:
		return runRecordClassifyAction(args[1:], stderr)
	case "collect":
		return runRecordCollect(args[1:], stdout, stderr)
	}
	fmt.Fprintf(stderr, "kiln: unknown record subcommand %q\n", args[0])
	return 2
}

// runRecordCollect is `kiln record collect --root <abs> --dir <abs> --results
// <abs> --out <abs>`.
func runRecordCollect(args []string, stdout, stderr io.Writer) int {
	fs := flag.NewFlagSet("record collect", flag.ContinueOnError)
	fs.SetOutput(stderr)
	root := fs.String("root", "", "the checkout (absolute)")
	dir := fs.String("dir", "", "the record directory prepare sealed (absolute)")
	results := fs.String("results", "", "the build tool's -emit-results file (absolute)")
	out := fs.String("out", "", "where records.jsonl is written; it must not exist (absolute)")
	if err := fs.Parse(args); err != nil {
		return 2
	}
	usage := "usage: kiln record collect --root <abs> --dir <abs> --results <abs> --out <abs>"
	if fs.NArg() != 0 || *root == "" || *dir == "" || *results == "" || *out == "" {
		fmt.Fprintln(stderr, usage)
		return 2
	}
	for _, p := range []struct{ name, v string }{{"--root", *root}, {"--dir", *dir}, {"--results", *results}, {"--out", *out}} {
		if !filepath.IsAbs(p.v) {
			fmt.Fprintf(stderr, "kiln: %s %q is not an absolute path\n", p.name, p.v)
			return 2
		}
	}
	if !recordPlatformOK() {
		fmt.Fprintf(stderr, "kiln: record collect runs only on linux/amd64 (this is %s/%s); nothing was written\n", runtime.GOOS, runtime.GOARCH)
		return 3
	}
	self, err := recordSelf()
	if err != nil {
		fmt.Fprintf(stderr, "kiln: cannot locate this binary: %v\n", err)
		return 1
	}
	res, err := recordCollect(record.CollectOptions{Root: *root, Dir: *dir, Results: *results, Out: *out, KilnExe: self})
	if err != nil {
		fmt.Fprintf(stderr, "kiln: record collect: %v\n", err)
		return 1
	}
	// Nothing printed here is a value from the environment, and every action name
	// and reason has been made printable already (they came from a trace).
	h := res.Header
	fmt.Fprintf(stdout, "ok: shard %d of %d collected: %d records in %s; void %v; tracked_changed_unexplained %d; post_inventory_unexplained %d\n",
		h.ShardIndex, h.ShardCount, res.Records, res.Out, h.Void, h.TrackedChangedUnexplained, h.PostInventoryUnexplained)
	for _, u := range res.Unknown {
		fmt.Fprintf(stdout, "unknown write set: %s\n", u)
	}
	// Why a post-build check failed (printable and bounded by Collect): the shard
	// cannot carry it, and a void shard has to say why in the log.
	for _, p := range res.PostProblems {
		fmt.Fprintf(stdout, "post-build problem: %s\n", p)
	}
	return 0
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

// runRecordClassifyAction is `kiln record classify-action --dir <abs> --action
// <name> --step N --data-limit B`: the classification child the step supervisor
// starts at the end of an action (internal/record/classify_child.go). It only
// writes `record.json` into the record directory, atomically and never over an
// existing one.
func runRecordClassifyAction(args []string, stderr io.Writer) int {
	fs := flag.NewFlagSet("record classify-action", flag.ContinueOnError)
	fs.SetOutput(stderr)
	dir := fs.String("dir", "", "the record directory (absolute)")
	action := fs.String("action", "", "the action")
	step := fs.Int("step", -1, "the step that ended the action")
	limit := fs.Int64("data-limit", 0, "RLIMIT_DATA in bytes (0: none)")
	if err := fs.Parse(args); err != nil {
		return 2
	}
	if *dir == "" || !filepath.IsAbs(*dir) || *action == "" || *step < 0 || *limit < 0 || fs.NArg() != 0 {
		fmt.Fprintln(stderr, "usage: kiln record classify-action --dir <abs> --action <name> --step N [--data-limit B] (internal to `kiln record step`)")
		return 2
	}
	if !recordPlatformOK() {
		fmt.Fprintf(stderr, "kiln: record classify-action runs only on linux/amd64 (this is %s/%s)\n", runtime.GOOS, runtime.GOARCH)
		return 3
	}
	if err := record.ClassifyChild(*dir, *action, *step, *limit, record.ChildHooks{}); err != nil {
		fmt.Fprintf(stderr, "kiln: record classify-action: %v\n", err)
		return 1
	}
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
	// The parent-death signal belongs to the thread that asks for it, and fires
	// when the build tool's forking thread dies: keep this goroutine on one OS
	// thread for the life of the process, so the request is made from, and
	// stays with, a thread that lives as long as the process does.
	runtime.LockOSThread()
	if err := recordArmParentDeath(); err != nil {
		fmt.Fprintf(stderr, "kiln: record step: %v\n", err)
		return record.ExitRefused
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
	// The epilogue's verdict never changes the status, but a failed
	// classification is worth one line in the build log (the reason is the
	// recorder's own text, printable).
	if res.Record == record.RecordFailed {
		fmt.Fprintf(stderr, "kiln: record step: no record.json for the action: %s\n", res.RecordWhy)
	}
	return res.ExitCode
}

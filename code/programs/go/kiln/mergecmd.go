package main

// `kiln record-merge` (KILN04 section 7.1, stage 4 PR 3a)
// =========================================================================
//
// One job joins every recording shard, applies the cross-shard rules once and
// writes the ledger. Until it ran, no shard's work could become a manifest or
// a pass: `kiln record collect` writes raw records and decides nothing, by
// design (section 7, rule 1).
//
// The whole verdict is `record.Merge`; this file is argument handling and the
// report. The exit status is the report:
//
//	0  the run emitted (or --dry-run computed a non-void verdict)
//	1  the job itself is broken: an input could not be read or decoded
//	2  usage
//	3  this run may not write the ledger it was asked for
//	4  the run is void: nothing was written, and the causes are in the report
//
// 4 is deliberately distinct from 1. A void run is a RESULT -- the merge
// refused what the shards submitted -- and the CI job must be able to tell
// "the recorder is not sound yet" from "the merge job is broken".

import (
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/host"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/ledger"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recmerge"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// mergeShardFlags collects the repeatable `--shard index=dir`.
type mergeShardFlags struct {
	arts []record.ShardArtifact
}

func (s *mergeShardFlags) String() string {
	var parts []string
	for _, a := range s.arts {
		parts = append(parts, fmt.Sprintf("%d=%s", a.Index, a.Dir))
	}
	return strings.Join(parts, ",")
}

// Set parses one `INDEX=DIR`. The index is what the artifact was NAMED, and it
// is never read from inside the artifact: a shard must not be able to claim to
// be another shard.
func (s *mergeShardFlags) Set(v string) error {
	i := strings.IndexByte(v, '=')
	if i <= 0 || i == len(v)-1 {
		return fmt.Errorf("--shard needs INDEX=DIR, not %q", printableArg(v))
	}
	n, err := strconv.Atoi(v[:i])
	if err != nil || n < 0 {
		return fmt.Errorf("--shard index %q is not a non-negative integer", printableArg(v[:i]))
	}
	s.arts = append(s.arts, record.ShardArtifact{Index: n, Dir: v[i+1:]})
	return nil
}

func runRecordMerge(args []string, stdout, stderr io.Writer) int {
	fs := flag.NewFlagSet("record-merge", flag.ContinueOnError)
	fs.SetOutput(stderr)
	root := fs.String("root", "", "the checkout at the revision every shard recorded (absolute)")
	gitBin := fs.String("git", "/usr/bin/git", "the git binary, run by absolute path")
	graph := fs.String("graph", "", "the build tool's graph export at that revision (absolute)")
	steps := fs.String("steps", "", "the build tool's steps export at that revision (absolute)")
	plan := fs.String("plan", "", "the build tool's build-plan export at that revision (absolute)")
	shardCount := fs.Int("shard-count", 0, "the shard count the plan and every shard must agree on")
	ledgerDir := fs.String("ledger", "", "the ledger directory to write (absolute)")
	scope := fs.String("scope", "", "shared (CI) or local (a developer's machine)")
	dryRun := fs.Bool("dry-run", false, "compute the verdict and write nothing")
	nowUnix := fs.Int64("now", 0, "stamp the records at this unix time (default: now)")
	kilnID := fs.String("kiln", "", "the ledger record's kiln id (default: this binary's)")
	repository := fs.String("repository", "", "owner/name (default: $GITHUB_REPOSITORY)")
	runID := fs.String("run-id", "", "the workflow run id (default: $GITHUB_RUN_ID)")
	runAttempt := fs.Int("run-attempt", 0, "the workflow run attempt (default: $GITHUB_RUN_ATTEMPT)")
	revision := fs.String("revision", "", "the commit every shard recorded (default: $GITHUB_SHA)")
	var shards mergeShardFlags
	fs.Var(&shards, "shard", "INDEX=DIR of one shard artifact (repeat once per shard)")
	if err := fs.Parse(args); err != nil {
		return 2
	}
	if fs.NArg() != 0 || fs.Arg(0) != "" {
		fmt.Fprintln(stderr, "kiln: record-merge takes no positional arguments")
		return 2
	}

	// Environment fallbacks, read here rather than inside record.Merge so the
	// job package stays a pure function of its options.
	env := func(name, have string) string {
		if have != "" {
			return have
		}
		return os.Getenv(name)
	}
	attempt := *runAttempt
	if attempt == 0 {
		if n, err := strconv.Atoi(os.Getenv("GITHUB_RUN_ATTEMPT")); err == nil {
			attempt = n
		}
	}
	if attempt == 0 {
		attempt = 1
	}
	if *nowUnix == 0 {
		*nowUnix = time.Now().Unix()
	}
	if *kilnID == "" {
		*kilnID = fmt.Sprintf("kiln/%s traced/%d recorder/%d", Version, legacy.TracedSemantics, legacy.RecorderVersion)
	}

	absRoot, err := filepath.Abs(env("", *root))
	if err != nil || *root == "" {
		fmt.Fprintln(stderr, "kiln: --root is required")
		return 2
	}
	repo, run, rev := env("GITHUB_REPOSITORY", *repository), env("GITHUB_RUN_ID", *runID), env("GITHUB_SHA", *revision)
	if repo == "" || run == "" || rev == "" {
		fmt.Fprintln(stderr, "kiln: the run's repository, run id and revision are required (flags, or $GITHUB_REPOSITORY, $GITHUB_RUN_ID and $GITHUB_SHA)")
		return 2
	}
	if len(shards.arts) == 0 || *shardCount < 1 || *graph == "" || *steps == "" || *plan == "" {
		fmt.Fprintln(stderr, "kiln: --graph, --steps, --plan, --shard-count and at least one --shard are required")
		return 2
	}
	if !*dryRun && (*ledgerDir == "" || *scope == "") {
		fmt.Fprintln(stderr, "kiln: --ledger and --scope are required unless --dry-run is set")
		return 2
	}
	var sc ledger.Scope
	switch *scope {
	case "shared":
		sc = ledger.Shared
	case "local":
		sc = ledger.Local
	case "":
		// --dry-run: the scope is never consulted.
	default:
		fmt.Fprintln(stderr, "kiln: --scope must be shared or local")
		return 2
	}

	// The trust verdict comes from the real Host adapters, never from a flag: a
	// `--trusted` switch would be exactly the stray `true` the Authority
	// interface exists to prevent.
	auth := host.Detect(os.Getenv, os.Environ())
	// Refuse an unauthorized write here, as exit 3, so the status says "this
	// run may not record" rather than "the job is broken". record.Merge
	// checks it again; the two answers cannot differ because both call
	// ledger.CheckAuthority.
	if !*dryRun {
		if err := ledger.CheckAuthority(auth, sc); err != nil {
			fmt.Fprintf(stderr, "kiln: this run may not write the %s ledger: %v\n", *scope, err)
			return 3
		}
	}

	rep, err := record.Merge(record.MergeOptions{
		Root:       absRoot,
		Source:     source.NewGitAt(absRoot, *gitBin, source.PinnedEnv()),
		GraphPath:  *graph,
		StepsPath:  *steps,
		PlanPath:   *plan,
		ShardCount: *shardCount,
		Shards:     shards.arts,
		Run: recmerge.Run{
			Repository: repo,
			ID:         run,
			Attempt:    attempt,
			Revision:   rev,
		},
		Kiln:      *kilnID,
		Now:       time.Unix(*nowUnix, 0).UTC(),
		LedgerDir: *ledgerDir,
		Scope:     sc,
		Authority: auth,
		DryRun:    *dryRun,
	})
	if err != nil {
		fmt.Fprintf(stderr, "kiln: %v\n", err)
		return 1
	}
	writeMergeReport(stdout, rep, *dryRun)
	if !rep.Verdict.Emitted {
		return 4
	}
	return 0
}

// writeMergeReport prints the verdict: the void causes first (they are the
// reason nothing was written), then the counts, then the reason histogram.
// Every string in it comes from recmerge and is already a fixed token or a
// package path; nothing is taken from a shard's free text without the sanitise
// recmerge applies.
func writeMergeReport(w io.Writer, rep *record.MergeReport, dryRun bool) {
	fmt.Fprintf(w, "revision %s, %d actions in the plan\n", rep.Revision, rep.PlanPackages)
	if !rep.Verdict.Emitted {
		fmt.Fprintf(w, "run VOID: %d cause(s), nothing written\n", rep.Verdict.RunVoidCount)
		for _, c := range rep.Verdict.RunVoid {
			fmt.Fprintf(w, "  void %s\n", c)
		}
		if rep.Verdict.RunVoidCount > len(rep.Verdict.RunVoid) {
			fmt.Fprintf(w, "  ... %d more not shown\n", rep.Verdict.RunVoidCount-len(rep.Verdict.RunVoid))
		}
	} else if dryRun {
		fmt.Fprintf(w, "run OK (dry run): %d manifest(s) and %d pass(es) would be written\n",
			rep.Manifests, rep.Passes)
	} else {
		fmt.Fprintf(w, "run OK: wrote %d manifest(s) and %d pass(es)\n", rep.Manifests, rep.Passes)
	}
	if len(rep.Verdict.ShardVoid) > 0 {
		idx := make([]int, 0, len(rep.Verdict.ShardVoid))
		for i := range rep.Verdict.ShardVoid {
			idx = append(idx, i)
		}
		sort.Ints(idx)
		for _, i := range idx {
			fmt.Fprintf(w, "  shard %d void: %s\n", i, strings.Join(rep.Verdict.ShardVoid[i], "; "))
		}
	}
	if rep.Language != "" {
		// The selection came from the shards' bindings. The drop count is a
		// number, never a path: the observed paths are not printed.
		fmt.Fprintf(w, "language selection %s: %d action(s) lost their record for reading an unselected package's untracked or absent paths\n",
			rep.Language, rep.ForeignDropped)
	}
	fmt.Fprintf(w, "recordable %d of %d action(s)\n", rep.Recordable, rep.Actions)
	if len(rep.Reasons) > 0 {
		fmt.Fprintln(w, "reasons:")
		for _, r := range rep.Reasons {
			fmt.Fprintf(w, "  %6d  %s\n", r.Count, r.Reason)
		}
	}
}

// printableArg quotes an argument for an error message without letting a
// control character through to the log. The argument is a command-line value,
// so it is short; the cap keeps a pathological one from flooding the job log.
func printableArg(s string) string {
	const max = 64
	if len(s) > max {
		s = s[:max] + "..."
	}
	var b strings.Builder
	b.WriteByte('"')
	for _, r := range s {
		if r < 0x20 || r == 0x7f {
			b.WriteByte('?')
			continue
		}
		b.WriteRune(r)
	}
	b.WriteByte('"')
	return b.String()
}

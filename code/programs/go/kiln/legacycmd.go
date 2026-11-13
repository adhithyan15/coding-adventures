package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/host"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/ledger"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// runLegacyPlan implements
//
//	kiln legacy-plan --graph <graph.json> --out <plan.json>
//	                 [--os linux|darwin|windows] [--root <dir>] [--source git|folder]
//	                 [--probe]
//	                 [--ledger <dir> --scope shared|local [--executor-arg <flag>]...]
//
// With --ledger the plan also carries the KILN04 stage 5a lookup report: for
// each Linux package, whether a traced pass would hit.  It is shadow only (it
// skips nothing) and is refused with --source folder, which cannot vouch for a
// clean tree.
//
// It computes the legacy key of every package in the graph (KILN03) and writes
// the plan atomically.  Any problem is an error and a non-zero exit: a plan
// that silently skipped a package would hide exactly the packages most in need
// of a look.
func runLegacyPlan(args []string, stdout, stderr io.Writer) int {
	fs := flag.NewFlagSet("legacy-plan", flag.ContinueOnError)
	fs.SetOutput(stderr)
	graphPath := fs.String("graph", "", "legacy graph export (kiln-legacy-graph/1)")
	outPath := fs.String("out", "", "plan file to write")
	osName := fs.String("os", runtime.GOOS, "platform to key for: linux, darwin or windows")
	root := fs.String("root", ".", "repository root")
	kind := fs.String("source", "git", "tree adapter: git or folder")
	probe := fs.Bool("probe", false, "run the toolchain probes of KILN03 4 and report disagreements (soft reasons; off by default)")
	ledgerDir := fs.String("ledger", "", "ledger directory to look traced passes up in (KILN04 stage 5a, shadow only)")
	scope := fs.String("scope", "", "with --ledger: shared or local")
	var execArgs stringList
	fs.Var(&execArgs, "executor-arg", "with --ledger: one flag of the recorder's executor, as keyed by `kiln:executor-args` (repeatable)")
	if err := fs.Parse(args); err != nil {
		return 2
	}
	usage := "usage: kiln legacy-plan --graph <graph.json> --out <plan.json> [--os linux|darwin|windows] [--root <dir>] [--source git|folder] [--probe] [--ledger <dir> --scope shared|local [--executor-arg <flag>]...]"
	if *graphPath == "" || *outPath == "" || fs.NArg() != 0 {
		fmt.Fprintln(stderr, usage)
		return 2
	}
	if (*ledgerDir == "") != (*scope == "") || (*ledgerDir == "" && len(execArgs) > 0) || (*ledgerDir != "" && *scope != "shared" && *scope != "local") {
		fmt.Fprintln(stderr, usage)
		return 2
	}
	if *ledgerDir != "" && *kind == "folder" {
		fmt.Fprintln(stderr, "kiln: refusing --ledger with --source folder: a plain folder cannot check that the tree is clean, "+
			"so a lookup could describe files nobody committed (use --source git)")
		return 2
	}
	var src legacy.Source
	switch *kind {
	case "git":
		src = source.NewGit(*root)
	case "folder":
		src = source.NewFolder(*root)
		fmt.Fprintln(stderr, "kiln: warning: --source folder cannot check that the tree is clean; "+
			"build products or other stray files in the folder will enter the keys (shadow mode continues)")
	default:
		fmt.Fprintf(stderr, "kiln: unknown --source %q (want git or folder)\n", *kind)
		return 2
	}
	arch, err := legacy.HostArch()
	if err != nil {
		fmt.Fprintf(stderr, "kiln: %v\n", err)
		return 1
	}
	g, err := legacy.ReadGraphFile(*graphPath)
	if err != nil {
		fmt.Fprintf(stderr, "kiln: %v\n", err)
		return 1
	}
	opt := legacy.Options{
		Source:  src,
		Host:    host.Detect(os.Getenv, os.Environ()),
		OS:      *osName,
		Arch:    arch,
		Environ: os.Environ(),
	}
	if *probe {
		// A probe runs the tools of THIS machine, so it can only speak for the
		// platform it runs on.
		if *osName != runtime.GOOS {
			fmt.Fprintf(stderr, "kiln: --probe needs --os %s (this machine); probes cannot speak for another platform\n", runtime.GOOS)
			return 2
		}
		abs, err := filepath.Abs(*root)
		if err != nil {
			fmt.Fprintf(stderr, "kiln: %v\n", err)
			return 1
		}
		opt.Probe = &legacy.ProbeOptions{Root: abs, Home: os.Getenv("HOME")}
	}
	if *ledgerDir != "" {
		abs, err := filepath.Abs(*root)
		if err != nil {
			fmt.Fprintf(stderr, "kiln: %v\n", err)
			return 1
		}
		sc := ledger.Local
		if *scope == "shared" {
			sc = ledger.Shared
		}
		opt.Lookup = legacy.NewLookupOptions(*ledgerDir, sc, time.Now().UTC(), abs, execArgs)
	}
	plan, err := legacy.BuildPlan(g, opt)
	if err != nil {
		fmt.Fprintf(stderr, "kiln: %v\n", err)
		return 1
	}
	data, err := plan.Marshal()
	if err != nil {
		fmt.Fprintf(stderr, "kiln: %v\n", err)
		return 1
	}
	if err := legacy.WriteFileAtomic(*outPath, data); err != nil {
		fmt.Fprintf(stderr, "kiln: %v\n", err)
		return 1
	}
	h := plan.Histogram
	fmt.Fprintf(stdout, "ok: %d packages (%s/%s); classes %v; closure size min %d median %d p90 %d max %d\n",
		len(plan.Packages), plan.OS, plan.Arch, h.Classes,
		h.ClosureSizes.Min, h.ClosureSizes.Median, h.ClosureSizes.P90, h.ClosureSizes.Max)
	if plan.Lookup != nil {
		fmt.Fprintf(stdout, "lookup (%s, shadow only): %v; %d package(s) read outside their static closure\n",
			plan.Lookup.Scope, h.Lookup, h.OutsideStaticClosure)
	}
	return 0
}

// stringList is a repeatable string flag.
type stringList []string

func (l *stringList) String() string     { return fmt.Sprint([]string(*l)) }
func (l *stringList) Set(v string) error { *l = append(*l, v); return nil }

// runExplain implements `kiln explain --plan <plan.json> <package>`: it prints
// the package's record, component digests included, as indented JSON.
func runExplain(args []string, stdout, stderr io.Writer) int {
	fs := flag.NewFlagSet("explain", flag.ContinueOnError)
	fs.SetOutput(stderr)
	planPath := fs.String("plan", "", "plan file written by legacy-plan")
	if err := fs.Parse(args); err != nil {
		return 2
	}
	if *planPath == "" || fs.NArg() != 1 {
		fmt.Fprintln(stderr, "usage: kiln explain --plan <plan.json> <package>")
		return 2
	}
	plan, err := legacy.ReadPlanFile(*planPath)
	if err != nil {
		fmt.Fprintf(stderr, "kiln: %v\n", err)
		return 1
	}
	rec, ok := plan.Find(fs.Arg(0))
	if !ok {
		fmt.Fprintf(stderr, "kiln: package %q is not in the plan\n", fs.Arg(0))
		return 1
	}
	// Hints are advisory (KILN03 section 6): they get their own section, apart
	// from the record's reasons, so no reader mistakes them for a verdict.
	type recordView struct {
		Name        string            `json:"name"`
		Key         string            `json:"key"`
		Class       string            `json:"class"`
		Reasons     []string          `json:"reasons"`
		ClosureSize int               `json:"closure_size"`
		Tools       []string          `json:"tools"`
		Components  legacy.Components `json:"components"`
	}
	hints := rec.Hints
	if hints == nil {
		hints = []string{}
	}
	out := struct {
		OS     string     `json:"os"`
		Arch   string     `json:"arch"`
		Host   string     `json:"host"`
		Record recordView `json:"record"`
		// Env names the variables keyed in every package; the values are
		// digested into record.components.env and never printed (KILN04 4.3).
		Env    []string             `json:"env"`
		Probes []legacy.ProbeResult `json:"probes,omitempty"`
		// Lookup is the KILN04 stage 5a result, present when the plan was made
		// with --ledger: the weak key, the manifests tried (entry lists
		// included), the hit's pass record and the traced packages outside the
		// static closure.  LookupInfo says what was read and that it is shadow.
		// Each manifest's entries are only what the plan holds (at most
		// legacy.MaxPlanEntries); LookupNote says when that is a truncation.
		Lookup     *legacy.Lookup     `json:"lookup,omitempty"`
		LookupNote string             `json:"lookup_note,omitempty"`
		LookupInfo *legacy.LookupInfo `json:"lookup_info,omitempty"`
		Advisory   struct {
			Note  string   `json:"note"`
			Hints []string `json:"hints"`
		} `json:"advisory_hints"`
	}{OS: plan.OS, Arch: plan.Arch, Host: plan.Host, Env: plan.Env, Probes: plan.Probes,
		Lookup: rec.Lookup, LookupInfo: plan.Lookup,
		Record: recordView{rec.Name, rec.Key, rec.Class, rec.Reasons, rec.ClosureSize, rec.Tools, rec.Components}}
	if rec.Lookup != nil {
		for _, m := range rec.Lookup.Manifests {
			if m.EntriesTruncated {
				out.LookupNote = fmt.Sprintf("some manifests list only the first %d of their entries (see entry_count); the plan does not hold the rest", legacy.MaxPlanEntries)
				break
			}
		}
	}
	if out.Env == nil {
		out.Env = []string{}
	}
	if rec.Tools == nil {
		out.Record.Tools = []string{}
	}
	out.Advisory.Note = "advisory only: static scans for triage and debugging; they make no soundness or security claim and do not affect the class"
	out.Advisory.Hints = hints
	enc := json.NewEncoder(stdout)
	enc.SetEscapeHTML(false)
	enc.SetIndent("", "  ")
	if err := enc.Encode(out); err != nil {
		fmt.Fprintf(stderr, "kiln: %v\n", err)
		return 1
	}
	return 0
}

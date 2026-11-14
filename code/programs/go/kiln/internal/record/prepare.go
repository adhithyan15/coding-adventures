package record

// Prepare: everything the shard recorder does before any BUILD (KILN04 section
// 4.8, stage 4b-1, decisions P1 to P10)
// ===========================================================================
//
// A recording shard has three phases. This is the first, and the only one that
// can fail "cleanly": nothing a BUILD wrote exists yet, so every question about
// the machine and the tree can still be answered truthfully.
//
//	prepare (here)           facts about the tree, the machine and the plan,
//	                         sealed in `pre/` before any BUILD runs
//	the build                the build tool runs each BUILD step behind
//	                         `kiln record step` (stage 4 PR 2b-2)
//	collect                  parse the traces, classify, write the shard file
//	                         (stage 4 PR 2b-2)
//
// The order inside Prepare matters; each step relies on the ones before it:
//
//	 1  options, trust of the paths     nothing is read or written yet
//	 2  VerifyTree                      the disk IS the commit (G5); the commit
//	                                    is kept as the binding's `revision`
//	 3  the binaries                    kiln and build-tool are outside the
//	                                    checkout; hash them (D15)
//	 4  Inventory, ScanTree             no untracked BUILD, no hard link (P5)
//	 5  create the record directory
//	 6  graph, steps and plan           by the out-of-tree build tool, from the
//	                                    tree just verified (D10, P6)
//	 7  host facts, weak keys, actions  key every package the shard runs (P7)
//	 8  sandbox configuration           generate, validate, scan the roots for
//	                                    sockets and FIFOs, CheckHost (P8, P9)
//	 9  binding, SEALED, read-only      written last, exactly as recfmt encodes
//	                                    them (P4, P10)
//
// A failure anywhere returns an error and the job fails. A partly written
// record directory is left as it is: it has no SEALED file, and every later
// command refuses a directory without one, so a half-prepared shard can never
// be recorded from.
//
// Platform. Prepare itself is portable Go so that it can be tested on any
// machine; `kiln record prepare` (recordcmd.go) refuses to start anywhere but a
// trusted GitHub run on linux/amd64. Everything that touches the outside world
// goes through a field of Options, so the unit tests need neither GitHub, nor a
// real build tool, nor strace.

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// ShardWeightsPath is the measured durations the plan's partition uses. It is
// tracked, so the merge job regenerates the same plan from the same file.
const ShardWeightsPath = "code/specs/data/ci-package-durations.json"

// ToolTimeout bounds one run of the build tool during prepare.
const ToolTimeout = 15 * time.Minute

// ToolsSchema is the schema of pre/tools.json.
const ToolsSchema = "kiln-record-tools/1"

// ToolsFile is pre/tools.json: the programs the recorder runs from the host
// side, by absolute path, and the PATH it passes them. `kiln record step` and
// `collect` (PR 2b-2) use exactly these and never look a program up on PATH.
type ToolsFile struct {
	Schema string `json:"schema"`
	Git    string `json:"git"`
	Strace string `json:"strace"`
	Path   string `json:"path"`
	// BuildToolEnv is the build tool's ENTIRE environment ("NAME=value"), an
	// allowlist (buildToolEnv). `record step` requires its inherited
	// environment, apart from the KILN_* variables the build tool adds, to equal
	// it exactly.
	BuildToolEnv []string `json:"build_tool_env"`
}

// ToolRun is one start of the build tool.
type ToolRun struct {
	Path    string
	Args    []string
	Dir     string
	Env     []string
	Timeout time.Duration
}

// GraphArgs are the build tool's arguments for the graph and steps exports
// (one discovery pass writes both, D10). Exported so the tests can run the real
// build tool with exactly the arguments `prepare` uses.
func GraphArgs(root, graphPath, stepsPath string) []string {
	return []string{"-root", root, "-emit-kiln-graph", graphPath, "-emit-kiln-steps", stepsPath}
}

// PlanArgs are the arguments for the sharded plan: every package, whatever the
// diff, split into count prerequisite-closed shards by the measured weights.
func PlanArgs(root string, count int, planPath string) []string {
	return []string{"-root", root, "-force", "-language", "all", "-shard-count", strconv.Itoa(count),
		"-shard-weights", ShardWeightsPath, "-emit-plan", planPath}
}

// Runner starts a tool and waits for it. A non-zero exit is an error.
type Runner func(ToolRun) error

// Options are the inputs of Prepare. Paths must be absolute and clean.
type Options struct {
	// Root is the checkout, Dir the record directory (made if absent),
	// BuildTool the build-tool binary, KilnExe this binary, Strace the
	// strace binary (run once, as `strace --version`) and Git the git binary
	// every host-side Git call uses. None is ever looked up on PATH.
	Root, Dir, BuildTool, KilnExe, Strace, Git string

	ShardIndex, ShardCount int
	// JobStart is the Unix time the job's first step stamped (§5.3, setup
	// stamps).
	JobStart int64

	// Image is the host's image id (host.ImageID). Getenv reads GITHUB_*,
	// RUNNER_TEMP and XDG_CONFIG_HOME; Environ is the process environment the
	// keys' scrubbed environment is derived from.
	Image   string
	Getenv  func(string) string
	Environ []string

	// Tmpdir is the step's TMPDIR; it defaults to legacy.RecordTmpdir, the
	// value that is keyed. Tests set it elsewhere.
	Tmpdir string
	// Now is the clock; it defaults to time.Now.
	Now func() time.Time

	// The seams. Each defaults to the real thing.
	RunTool       Runner
	StraceVersion func(path string) (string, error)
	Kernel        func() (string, error)
}

// PrepareResult is what a successful Prepare reports.
type PrepareResult struct {
	Binding   recfmt.Binding
	BindingID string
	// PreDir is `<dir>/pre`.
	PreDir string
	// ExecWrapper is the value of the build tool's -exec-wrapper.
	ExecWrapper []string
	// Actions is the number of packages the shard runs.
	Actions int
	// Refused counts those among them that have no weak key.
	Refused int
}

// Prepare runs the whole of `kiln record prepare`.
func Prepare(o Options) (*PrepareResult, error) {
	if err := o.normalize(); err != nil {
		return nil, err
	}
	getenv := o.Getenv
	repo, run, attempt, err := githubIdentity(getenv)
	if err != nil {
		return nil, err
	}
	home, err := homeOf(o.Environ)
	if err != nil {
		return nil, err
	}

	// 1. The record directory argument is checked before anything is read.
	create, err := checkDirArg(o.Dir)
	if err != nil {
		return nil, err
	}
	kiln, err := ResolveBinary("the kiln binary", o.KilnExe)
	if err != nil {
		return nil, err
	}
	bt, err := ResolveBinary("--build-tool", o.BuildTool)
	if err != nil {
		return nil, err
	}
	strace, err := ResolveBinary("--strace", o.Strace)
	if err != nil {
		return nil, err
	}
	git, err := ResolveBinary("--git", o.Git)
	if err != nil {
		return nil, err
	}
	if _, err := Canonical("--root", o.Root); err != nil {
		return nil, err
	}
	early := []Region{{"the checkout", o.Root}, {"tmpdir", o.Tmpdir}}
	for _, b := range []struct{ what, p string }{
		{"the kiln binary", kiln.Path}, {"--build-tool", bt.Path}, {"--strace", strace.Path}, {"--git", git.Path}, {"--dir", o.Dir},
	} {
		if err := CheckOutside(b.what, b.p, early); err != nil {
			return nil, err
		}
	}
	// The recorder's own PATH (decision P11): the build tool starts programs
	// through it, so it must hold nothing a step could have written.
	safe, err := SafePath(pathOf(o.Environ), append(append([]Region{}, early...), Region{"the record directory", o.Dir}))
	if err != nil {
		return nil, err
	}
	// The dynamic loader and C library variables (M3): a library planted in the
	// checkout must not be loaded by anything the recorder starts.
	loader, loaderDirs, err := LoaderEnv(o.Environ, append(append([]Region{}, early...), Region{"the record directory", o.Dir}))
	if err != nil {
		return nil, err
	}
	xdg := getenv("XDG_CONFIG_HOME")
	if xdg != "" && !filepath.IsAbs(xdg) {
		return nil, fmt.Errorf("XDG_CONFIG_HOME %q is not an absolute path", printable(xdg))
	}
	// Every host-side Git call runs the one git resolved above, by path, with an
	// ALLOWLIST environment: HOME, the validated PATH and XDG_CONFIG_HOME, and
	// the fixed hardening source.NewGitAt adds. Nothing else of the process
	// environment reaches git.
	gitEnv := []string{"HOME=" + home, "PATH=" + safe.String()}
	if xdg != "" {
		gitEnv = append(gitEnv, "XDG_CONFIG_HOME="+xdg)
	}
	src := source.NewGitAt(o.Root, git.Path, gitEnv)

	// 2. The disk is the commit, and HEAD is the commit GitHub is running.
	vt, err := VerifyTreeWith(src, o.Root, "")
	if err != nil {
		return nil, fmt.Errorf("verify tree: %w", err)
	}
	if want := strings.ToLower(getenv("GITHUB_SHA")); vt.Commit != want {
		return nil, fmt.Errorf("HEAD is %s but GITHUB_SHA is %q: the checkout is not the commit this run is for", short(vt.Commit), printable(want))
	}

	// 3. Nothing untracked may change what runs, and no file has a second name.
	inv, err := InventoryWith(src, o.Root)
	if err != nil {
		return nil, fmt.Errorf("inventory: %w", err)
	}
	if err := CheckInventory(inv); err != nil {
		return nil, err
	}
	if err := ScanTreeWith(src, o.Root); err != nil {
		return nil, err
	}

	// 4. The record directory exists from here on.
	pre := filepath.Join(o.Dir, "pre")
	if create {
		if err := os.Mkdir(o.Dir, 0o700); err != nil {
			return nil, fmt.Errorf("create --dir: %w", err)
		}
	}
	// traces/ is created here, once, 0700, so that the supervisors of steps
	// that start together (the build tool runs independent packages in
	// parallel) never race to make it. It is outside pre/, so SEALED does not
	// list it.
	for _, d := range []string{pre, filepath.Join(pre, "actions"), filepath.Join(o.Dir, "traces")} {
		if err := os.Mkdir(d, 0o700); err != nil {
			return nil, fmt.Errorf("create %s: %w", d, err)
		}
	}
	w := &sealWriter{pre: pre, sums: map[string]string{}}

	// 5. Graph, steps and plan, by the out-of-tree build tool.
	toolEnv := buildToolEnv(home, safe.String(), loader)
	graphPath, stepsPath, planPath := filepath.Join(pre, "graph.json"), filepath.Join(pre, "steps.json"), filepath.Join(pre, "build-plan.json")
	if err := o.RunTool(ToolRun{Path: bt.Path, Dir: o.Dir, Env: toolEnv, Timeout: ToolTimeout, Args: GraphArgs(o.Root, graphPath, stepsPath)}); err != nil {
		return nil, fmt.Errorf("build tool (graph and steps export): %w", err)
	}
	if err := o.RunTool(ToolRun{Path: bt.Path, Dir: o.Dir, Env: toolEnv, Timeout: ToolTimeout, Args: PlanArgs(o.Root, o.ShardCount, planPath)}); err != nil {
		return nil, fmt.Errorf("build tool (plan export): %w", err)
	}
	// Each export is read ONCE: the bytes that are hashed for the binding are
	// the bytes that are parsed.
	graphData, err := w.adopt("graph.json", legacy.MaxGraphBytes)
	if err != nil {
		return nil, err
	}
	stepsData, err := w.adopt("steps.json", legacy.MaxStepsBytes)
	if err != nil {
		return nil, err
	}
	planData, err := w.adopt("build-plan.json", planLimit)
	if err != nil {
		return nil, err
	}
	graph, err := legacy.ParseGraph(graphData)
	if err != nil {
		return nil, err
	}
	steps, err := legacy.DecodeSteps(stepsData)
	if err != nil {
		return nil, err
	}
	if steps.OS != "linux" {
		return nil, fmt.Errorf("steps export is for %q, not linux", printable(steps.OS))
	}
	part, err := parsePartition(planData, o.ShardCount)
	if err != nil {
		return nil, err
	}
	partDigest, err := part.Digest()
	if err != nil {
		return nil, err
	}
	runs, err := part.Runs(o.ShardIndex)
	if err != nil {
		return nil, err
	}

	// 6. Host facts, then the weak key of every package the shard runs.
	envEntries, err := legacy.RecordEnviron(src, o.Environ, o.Tmpdir)
	if err != nil {
		return nil, err
	}
	facts := recfmt.HostFacts{
		Schema: recfmt.SchemaHostFacts, Image: o.Image, Root: o.Root, Home: home, Tmpdir: o.Tmpdir,
		ExecutorArgs: append([]string(nil), legacy.RecordExecutorArgs...), Env: envEntries,
	}
	factsBytes, err := recfmt.EncodeHostFacts(facts)
	if err != nil {
		return nil, err
	}
	arch, err := legacy.HostArch()
	if err != nil {
		return nil, err
	}
	traced, err := legacy.NewTraced(graph, legacy.Options{
		Source: src, Host: facts, OS: "linux", Arch: arch, Environ: facts.Environ(),
	}, o.Root, legacy.RecordExecutorArgs)
	if err != nil {
		return nil, err
	}
	langOf := make(map[string]string, len(graph.Packages))
	for _, p := range graph.Packages {
		langOf[p.RelPath] = p.Language
	}
	envByAction := make(map[string][]string, len(runs))
	actions := make([]Action, 0, len(runs))
	refused := 0
	for _, rel := range runs {
		lang, ok := langOf[rel]
		if !ok {
			return nil, fmt.Errorf("the plan runs %q, which the graph export does not know", printable(rel))
		}
		lines, _ := steps.Lines(rel) // a package with no Linux BUILD runs nothing
		a := Action{Schema: ActionSchema, Action: rel, Language: lang, Steps: append([]string{}, lines...)}
		wk, err := traced.WeakKey(rel)
		if ref, isRef := legacy.IsRefusal(err); isRef {
			a.Refusal = ref.Reason
			refused++
			env, _, err := traced.RecordEnv(rel)
			if err != nil {
				return nil, fmt.Errorf("environment of %q: %w", printable(rel), err)
			}
			envByAction[rel] = legacy.EnvironOf(env)
		} else if err != nil {
			return nil, fmt.Errorf("weak key of %q: %w", printable(rel), err)
		} else {
			a.Weak = hex.EncodeToString(wk.Key[:])
			envByAction[rel] = wk.Environ()
		}
		actions = append(actions, a)
	}

	// 7. The sandbox configuration, scanned and checked on this host.
	if err := ensureTmpdir(o.Tmpdir); err != nil {
		return nil, err
	}
	cfg, err := BuildSandboxConfig(ConfigInput{
		Root: o.Root, Home: home, Tmpdir: o.Tmpdir, KilnExe: kiln.Path,
		XDGConfigHome: xdg, Roots: traced.HostRoots(), EnvByAction: envByAction,
		Private: privateRegions(o, kiln.Path, bt.Path), PrivateState: append(
			[]Region{{"the strace binary", strace.Path}, {"the git binary", git.Path}}, append(safe.Regions(), loaderDirs...)...),
	})
	if err != nil {
		return nil, err
	}
	inner := []Region{{"the checkout", o.Root}, {"tmpdir", o.Tmpdir}}
	for _, r := range cfg.RoRoots {
		inner = append(inner, Region{"an ro_roots entry", r})
	}
	for _, r := range cfg.StateRoots {
		inner = append(inner, Region{"a state root", r})
	}
	for _, b := range []struct{ what, p string }{{"the kiln binary", kiln.Path}, {"--build-tool", bt.Path}} {
		if err := CheckOutside(b.what, b.p, inner); err != nil {
			return nil, err
		}
	}
	// strace and git are the image's own programs and normally live in a
	// read-only root (/usr/bin), which a step can read but not write; they must
	// only stay out of what a step can WRITE.
	writable := []Region{{"the checkout", o.Root}, {"tmpdir", o.Tmpdir}}
	for _, r := range cfg.StateRoots {
		writable = append(writable, Region{"a state root", r})
	}
	for _, b := range []struct{ what, p string }{{"--strace", strace.Path}, {"--git", git.Path}, {"--dir", o.Dir}} {
		if err := CheckOutside(b.what, b.p, writable); err != nil {
			return nil, err
		}
	}
	if err := HostScan(cfg.RoRoots); err != nil {
		return nil, err
	}
	cfgBytes, err := MarshalConfig(cfg)
	if err != nil {
		return nil, err
	}
	if _, err := sandbox.ParseConfig(cfgBytes); err != nil {
		return nil, fmt.Errorf("the generated sandbox configuration does not parse: %w", err)
	}

	// 8. Write everything, the binding and SEALED last.
	invBytes, err := recfmt.EncodeInventory(inv)
	if err != nil {
		return nil, err
	}
	wrapper := []string{kiln.Path, "record", "step", "--dir", o.Dir, "--"}
	wrapperBytes, err := marshalJSON(wrapper)
	if err != nil {
		return nil, err
	}
	// The programs the recorder runs, by absolute path, and the PATH it passes:
	// `kiln record step` and `collect` use exactly these and never look
	// anything up (decision P11).
	toolsBytes, err := marshalJSON(ToolsFile{Schema: ToolsSchema, Git: git.Path, Strace: strace.Path, Path: safe.String(), BuildToolEnv: toolEnv})
	if err != nil {
		return nil, err
	}
	straceVer, err := o.StraceVersion(strace.Path)
	if err != nil {
		return nil, err
	}
	kernel, err := o.Kernel()
	if err != nil {
		return nil, err
	}
	for _, f := range []struct {
		rel  string
		data []byte
	}{{"host-facts.json", factsBytes}, {"inventory.json", invBytes}, {"sandbox.json", cfgBytes}, {"exec-wrapper.json", wrapperBytes}, {"tools.json", toolsBytes}} {
		if err := w.write(f.rel, f.data); err != nil {
			return nil, err
		}
	}
	// The sandbox's own host check, on the file as written: every path
	// canonical, every mask present, the configuration where no step can reach
	// it. It is what `kiln sandbox-exec` will do for each step, done once now,
	// so a configuration the helper would refuse fails the job here.
	if len(actions) > 0 {
		first := actions[0].Action
		err := cfg.CheckHost(first, sandbox.HostFacts{Exe: kiln.Path, Cwd: cfg.PackageDir(first), ConfigPath: filepath.Join(pre, "sandbox.json")})
		if err != nil {
			return nil, fmt.Errorf("%w (the record directory must lie outside home, tmpdir, the checkout and every state root)", err)
		}
	}
	for _, a := range actions {
		data, err := EncodeAction(a)
		if err != nil {
			return nil, err
		}
		if err := w.write(ActionFile(a.Action), data); err != nil {
			return nil, err
		}
	}
	binding := recfmt.Binding{
		Schema: recfmt.SchemaBinding, Repository: repo, Run: run, Attempt: attempt, Revision: vt.Commit,
		ShardIndex: o.ShardIndex, ShardCount: o.ShardCount,
		GraphSHA256: w.sums["graph.json"], StepsSHA256: w.sums["steps.json"], PartitionSHA256: partDigest,
		HostFactsSHA256: w.sums["host-facts.json"], InventorySHA256: w.sums["inventory.json"],
		SandboxConfigSHA256: w.sums["sandbox.json"], KilnSHA256: kiln.SHA256, BuildToolSHA256: bt.SHA256, StraceSHA256: strace.SHA256, GitSHA256: git.SHA256,
		Recorder: legacy.RecorderVersion, StraceVersion: straceVer, Kernel: kernel, JobStart: o.JobStart,
		PreparedAt: o.Now().UTC().Format(time.RFC3339),
	}
	bindBytes, err := recfmt.EncodeBinding(binding)
	if err != nil {
		return nil, err
	}
	if err := w.write("binding.json", bindBytes); err != nil {
		return nil, err
	}
	if err := w.seal(); err != nil {
		return nil, err
	}
	sum := sha256.Sum256(bindBytes)
	return &PrepareResult{Binding: binding, BindingID: hex.EncodeToString(sum[:]), PreDir: pre, ExecWrapper: wrapper,
		Actions: len(actions), Refused: refused}, nil
}

// normalize fills defaults and checks the options that need no file system.
func (o *Options) normalize() error {
	for _, p := range []struct{ name, v string }{
		{"--root", o.Root}, {"--dir", o.Dir}, {"--build-tool", o.BuildTool}, {"the kiln binary", o.KilnExe}, {"--strace", o.Strace}, {"--git", o.Git},
	} {
		if !filepath.IsAbs(p.v) || filepath.Clean(p.v) != p.v {
			return fmt.Errorf("%s %q must be an absolute, clean path", p.name, printable(p.v))
		}
	}
	if o.ShardCount < 1 || o.ShardCount > recfmt.MaxShards || o.ShardIndex < 0 || o.ShardIndex >= o.ShardCount {
		return fmt.Errorf("shard %d of %d is out of range", o.ShardIndex, o.ShardCount)
	}
	if o.JobStart < 0 {
		return fmt.Errorf("--job-start %d is negative", o.JobStart)
	}
	if o.Getenv == nil {
		o.Getenv = func(string) string { return "" }
	}
	if o.Tmpdir == "" {
		o.Tmpdir = legacy.RecordTmpdir
	}
	if o.Now == nil {
		o.Now = time.Now
	}
	if o.RunTool == nil {
		o.RunTool = execTool
	}
	if o.StraceVersion == nil {
		o.StraceVersion = straceVersion
	}
	if o.Kernel == nil {
		o.Kernel = kernelRelease
	}
	return nil
}

// githubIdentity reads the three variables that name this run.
func githubIdentity(getenv func(string) string) (repo, run string, attempt int, err error) {
	repo, run = getenv("GITHUB_REPOSITORY"), getenv("GITHUB_RUN_ID")
	attempt, aerr := strconv.Atoi(getenv("GITHUB_RUN_ATTEMPT"))
	if repo == "" || run == "" || aerr != nil || attempt < 1 {
		return "", "", 0, fmt.Errorf("GITHUB_REPOSITORY, GITHUB_RUN_ID and GITHUB_RUN_ATTEMPT must name this run (the binding is made of them)")
	}
	return repo, run, attempt, nil
}

// homeOf finds HOME in the environment (the LAST entry wins, as in os/exec) and
// requires an absolute, clean path.
func homeOf(environ []string) (string, error) {
	home := ""
	for _, e := range environ {
		if v, ok := strings.CutPrefix(e, "HOME="); ok {
			home = v
		}
	}
	if home == "" || !path.IsAbs(home) || path.Clean(home) != home || home == "/" {
		return "", fmt.Errorf("HOME %q must be an absolute, clean path other than /: the sandbox's $HOME and the toolchain roots beneath it come from it", printable(home))
	}
	return home, nil
}

// privateRegions are the places no ro root may overlap (sandboxcfg.go, 7).
func privateRegions(o Options, kiln, bt string) []Region {
	rs := []Region{{"the record directory", o.Dir}, {"the kiln binary", kiln}, {"--build-tool", bt}}
	if t := o.Getenv("RUNNER_TEMP"); t != "" {
		rs = append(rs, Region{"RUNNER_TEMP", path.Clean(t)})
	}
	for _, name := range []string{"GITHUB_ENV", "GITHUB_PATH", "GITHUB_OUTPUT", "GITHUB_STEP_SUMMARY", "GITHUB_EVENT_PATH"} {
		if f := o.Getenv(name); f != "" {
			rs = append(rs, Region{"the directory of " + name, path.Dir(path.Clean(f))})
		}
	}
	return rs
}

// ensureTmpdir creates the step's TMPDIR if it is absent and requires an
// existing one to be an empty real directory.
func ensureTmpdir(p string) error {
	st, err := os.Lstat(p)
	switch {
	case os.IsNotExist(err):
		if err := os.Mkdir(p, 0o700); err != nil {
			return fmt.Errorf("create tmpdir: %w", err)
		}
		return nil
	case err != nil:
		return fmt.Errorf("tmpdir %q: %v", printable(p), err)
	case st.Mode()&os.ModeSymlink != 0 || !st.IsDir():
		return fmt.Errorf("tmpdir %q is not a real directory", printable(p))
	}
	entries, err := os.ReadDir(p)
	if err != nil {
		return fmt.Errorf("tmpdir %q: %v", printable(p), err)
	}
	if len(entries) != 0 {
		return fmt.Errorf("tmpdir %q is not empty: the step's TMPDIR must start fresh", printable(p))
	}
	return claimDir("tmpdir", p, st)
}

// sealWriter writes the files of `pre/` exactly once each and remembers their
// digests for the binding and for SEALED.
type sealWriter struct {
	pre  string
	sums map[string]string // relative path -> hex SHA-256
}

// write creates pre/rel with O_EXCL (a file that exists is a bug or an attack,
// never overwritten) and records its digest.
func (w *sealWriter) write(rel string, data []byte) error {
	f, err := os.OpenFile(filepath.Join(w.pre, filepath.FromSlash(rel)), os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return err
	}
	if _, err := f.Write(data); err != nil {
		f.Close()
		return err
	}
	if err := f.Close(); err != nil {
		return err
	}
	sum := sha256.Sum256(data)
	w.sums[rel] = hex.EncodeToString(sum[:])
	return nil
}

// adopt reads a file another program wrote into pre/ ONCE, records the digest
// of exactly the bytes it returns, and returns them (so what is hashed is what
// is parsed). It must be a regular file (not a symlink, not a FIFO) of at most
// limit bytes.
func (w *sealWriter) adopt(rel string, limit int64) ([]byte, error) {
	full := filepath.Join(w.pre, filepath.FromSlash(rel))
	f, err := openNoFollow(full)
	if err != nil {
		return nil, fmt.Errorf("%s: %w", rel, err)
	}
	defer f.Close()
	st, err := f.Stat()
	if err != nil {
		return nil, err
	}
	if !st.Mode().IsRegular() {
		return nil, fmt.Errorf("%s is not a regular file", rel)
	}
	data, err := io.ReadAll(io.LimitReader(f, limit+1))
	if err != nil {
		return nil, err
	}
	if int64(len(data)) > limit {
		return nil, fmt.Errorf("%s exceeds %d bytes", rel, limit)
	}
	sum := sha256.Sum256(data)
	w.sums[rel] = hex.EncodeToString(sum[:])
	return data, nil
}

// seal writes SEALED, the digest of every other file of pre/ in path order, and
// then makes the tree read-only: files 0444, directories 0555.
func (w *sealWriter) seal() error {
	names := make([]string, 0, len(w.sums))
	for n := range w.sums {
		names = append(names, n)
	}
	sort.Strings(names)
	var b strings.Builder
	for _, n := range names {
		fmt.Fprintf(&b, "%s  %s\n", w.sums[n], n)
	}
	// Nothing may be in pre/ that SEALED does not list: a stray file (or a
	// symlink, which chmod would follow) would be sealed as if it were ours.
	if err := w.checkContents(); err != nil {
		return err
	}
	if err := w.write("SEALED", []byte(b.String())); err != nil {
		return err
	}
	if err := w.readOnly(); err != nil {
		// A directory that is not sealed must not look sealed. Directories that
		// were already made 0555 must be writable again for the remove to work.
		w.writable()
		if rerr := os.Remove(filepath.Join(w.pre, "SEALED")); rerr != nil {
			return errors.Join(err, fmt.Errorf("and SEALED could not be removed: %w", rerr))
		}
		return err
	}
	return nil
}

// checkContents requires every entry of pre/ to be the root, the actions
// directory, or a regular file whose digest is known.
func (w *sealWriter) checkContents() error {
	return filepath.WalkDir(w.pre, func(p string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, _ := filepath.Rel(w.pre, p)
		rel = filepath.ToSlash(rel)
		switch {
		case d.Type()&os.ModeSymlink != 0:
			return fmt.Errorf("pre/%s is a symlink", rel)
		case d.IsDir():
			if rel != "." && rel != "actions" {
				return fmt.Errorf("pre/%s is an unexpected directory", rel)
			}
		case !d.Type().IsRegular():
			return fmt.Errorf("pre/%s is not a regular file", rel)
		default:
			if _, ok := w.sums[rel]; !ok {
				return fmt.Errorf("pre/%s is not one of the files this run wrote", rel)
			}
		}
		return nil
	})
}

// writable undoes readOnly's directory modes (best effort, deepest last is not
// needed: parents first so children can be reached).
func (w *sealWriter) writable() {
	filepath.WalkDir(w.pre, func(p string, d os.DirEntry, err error) error {
		if err == nil && d.IsDir() {
			os.Chmod(p, 0o700)
		}
		return nil
	})
}

// readOnly makes the tree read-only: files 0444, directories 0555, deepest
// first. The modes are a convenience: integrity comes from the files being
// unreachable from the sandbox, and from `step` and `collect` re-verifying
// pre/ against SEALED (KILN04 4.8, 2b-2).
func (w *sealWriter) readOnly() error {
	var dirs []string
	err := filepath.WalkDir(w.pre, func(p string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.Type()&os.ModeSymlink != 0 {
			return fmt.Errorf("%s is a symlink", p)
		}
		if d.IsDir() {
			dirs = append(dirs, p)
			return nil
		}
		return os.Chmod(p, 0o444)
	})
	if err != nil {
		return err
	}
	for i := len(dirs) - 1; i >= 0; i-- {
		if err := os.Chmod(dirs[i], 0o555); err != nil {
			return err
		}
	}
	return nil
}

// marshalJSON renders v the way every pre/ file is: compact, no HTML escaping,
// no trailing newline.
func marshalJSON(v any) ([]byte, error) {
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(v); err != nil {
		return nil, err
	}
	return bytes.TrimSuffix(buf.Bytes(), []byte("\n")), nil
}

// execTool is the real Runner. Its output is kept (capped) for the error.
func execTool(r ToolRun) error {
	ctx, cancel := context.WithTimeout(context.Background(), r.Timeout)
	defer cancel()
	cmd := exec.CommandContext(ctx, r.Path, r.Args...)
	cmd.Dir, cmd.Env = r.Dir, r.Env
	// A tool that was killed at the deadline may have left a child holding its
	// output pipes; do not wait for them forever.
	cmd.WaitDelay = 10 * time.Second
	var out capBuffer
	cmd.Stdout, cmd.Stderr = &out, &out
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("%s: %w: %s", filepath.Base(r.Path), err, strings.TrimSpace(printable(out.String())))
	}
	return nil
}

// capBuffer keeps the first 4 KiB written to it and drops the rest.
type capBuffer struct{ b bytes.Buffer }

func (c *capBuffer) Write(p []byte) (int, error) {
	if room := 4096 - c.b.Len(); room > 0 {
		c.b.Write(p[:min(room, len(p))])
	}
	return len(p), nil
}
func (c *capBuffer) String() string { return c.b.String() }

// straceVersion runs `strace --version` and returns its first line, e.g.
// "strace -- version 6.8".
func straceVersion(p string) (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, p, "--version")
	cmd.Env = []string{"LANG=C", "PATH=/usr/bin:/bin"}
	cmd.Dir = string(os.PathSeparator) // never the checkout
	cmd.WaitDelay = 10 * time.Second
	var out capBuffer
	cmd.Stdout = &out
	if err := cmd.Run(); err != nil {
		return "", fmt.Errorf("%s --version: %w", p, err)
	}
	line, _, _ := strings.Cut(out.String(), "\n")
	line = strings.TrimSpace(line)
	if line == "" {
		return "", fmt.Errorf("%s --version printed nothing", p)
	}
	return line, nil
}

// kernelRelease is the running kernel's release string.
func kernelRelease() (string, error) {
	data, err := os.ReadFile("/proc/sys/kernel/osrelease")
	if err != nil {
		return "", fmt.Errorf("kernel release: %w", err)
	}
	return strings.TrimSpace(string(data)), nil
}

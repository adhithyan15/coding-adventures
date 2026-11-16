package warrantcorpus

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
)

// # The warrant-corpus command line (VER04i sections 5.3 and 6.2)
//
// The runner is a library: a package's entry program (for ct-compare,
// cmd/warrant-evidence) compiles in its registry and calls Main with the process
// arguments:
//
//	warrant-evidence <command> --repo-root R --corpus DIR --binding FILE
//	    --package-dir DIR [command options]
//
//	command               what it does
//	run [--out F]         full run; writes the manifest on a clean pass
//	check F               full run; fails if the regenerated manifest differs
//	sabotage              every control, in a child process, must print `not ok`
//	list-faults           <fault>:<op> per control
//	source-digest D       warrant-src-v1 digest of a source tree
//	validate-manifest F   structural validation of a manifest against the corpus
//
// Exit status: 0 everything passed, 1 a verdict failed, 2 the run was refused
// (bad corpus, bad binding, bad usage). Output is TAP-style: `ok <id>` /
// `not ok <id> <failure-id>`.
//
// The parser is hand-written on purpose: the runner has no dependencies, and a
// flag it does not know is a usage error, not something to ignore.

// LabelsFile is the label sidecar's file name, written beside the manifest.
const LabelsFile = "domain-labels.json"

var (
	commands   = []string{"run", "check", "sabotage", "list-faults", "source-digest", "validate-manifest"}
	valueFlags = []string{"--repo-root", "--corpus", "--binding", "--package-dir", "--out", "--registry"}
	commonFlag = []string{"--repo-root", "--corpus", "--binding", "--package-dir"}
)

// IO is where output goes and what the process looks like (a test passes its own).
type IO struct {
	Out io.Writer
	Err io.Writer
	// Env is the environment as KEY=VALUE pairs (os.Environ()).
	Env []string
	// Exe and ExeArgs are the command that re-runs this program, used by
	// `sabotage` to start one child per control. Exe == "" means the running
	// executable (os.Executable()).
	Exe     string
	ExeArgs []string
}

// ProcessIO is the real process: stdout, stderr and the real environment.
func ProcessIO() IO { return IO{Out: os.Stdout, Err: os.Stderr, Env: os.Environ()} }

type args struct {
	command       string
	flags         map[string]string
	requireActive bool
	positional    []string
}

func parseArgs(argv []string) *args {
	if len(argv) == 0 || !containsString(commands, argv[0]) {
		usageFail("usage", "expected one of: %s", strings.Join(commands, ", "))
	}
	parsed := &args{command: argv[0], flags: map[string]string{}}
	rest := argv[1:]
	for index := 0; index < len(rest); index++ {
		token := rest[index]
		switch {
		case token == "--require-active":
			parsed.requireActive = true
		case containsString(valueFlags, token):
			_, duplicate := parsed.flags[token]
			if index+1 >= len(rest) || duplicate {
				usageFail("usage", "%s needs one value", token)
			}
			parsed.flags[token] = rest[index+1]
			index++
		case strings.HasPrefix(token, "--"):
			usageFail("usage", "unknown option %s", token)
		default:
			parsed.positional = append(parsed.positional, token)
		}
	}
	wantsPositional := parsed.command == "check" || parsed.command == "source-digest" || parsed.command == "validate-manifest"
	want := 0
	if wantsPositional {
		want = 1
	}
	if len(parsed.positional) != want {
		quantity := "no"
		if wantsPositional {
			quantity = "exactly one"
		}
		usageFail("usage", "`%s` takes %s positional argument", parsed.command, quantity)
	}
	if parsed.command != "source-digest" {
		for _, required := range []string{"--repo-root", "--corpus"} {
			if _, has := parsed.flags[required]; !has {
				usageFail("usage", "%s is required", required)
			}
		}
	}
	return parsed
}

func (a *args) runConfig(fault, faultOp, filter string) RunConfig {
	binding, hasBinding := a.flags["--binding"]
	packageDir, hasPackage := a.flags["--package-dir"]
	if !hasBinding || !hasPackage {
		usageFail("usage", "--binding and --package-dir are required")
	}
	return RunConfig{
		RepoRoot:    a.flags["--repo-root"],
		CorpusDir:   a.flags["--corpus"],
		BindingPath: binding,
		PackageDir:  packageDir,
		Fault:       fault,
		FaultOp:     faultOp,
		Filter:      filter,
	}
}

// labelsFor derives the coverage-label sidecar, bound to the manifest by digest.
//
// The digest ignores toolchain.version (diagnostic, VER04i 8), exactly as `check`
// does, so a different Go patch release is not label drift. It does NOT ignore
// toolchain.name: swapping the toolchain is drift, and the name is read from the
// manifest being checked, never hard-coded (every lane's runner must accept every
// other lane's manifest).
func labelsFor(corpus *Corpus, manifest map[string]any) map[string]any {
	packageName, _ := manifest["package"].(string)
	module, _ := manifest["module"].(string)
	return deriveLabels(corpus, packageName, module, manifestLabelDigest(manifest))
}

// manifestLabelDigest is the SHA-256 of the canonical rendering of manifest with
// toolchain.version blanked (VER04i 9.1, label digest). The shared conformance
// fixture drives this function directly.
func manifestLabelDigest(manifest map[string]any) string {
	stable := map[string]any{}
	for key, value := range manifest {
		stable[key] = value
	}
	name := ""
	if toolchain, ok := manifest["toolchain"].(map[string]any); ok {
		name, _ = toolchain["name"].(string)
	}
	stable["toolchain"] = map[string]any{"name": name, "version": ""}
	return sha256Hex([]byte(renderJSON(stable)))
}

func writeLine(w io.Writer, line string) { fmt.Fprintln(w, line) }

func runCommand(a *args, stdio IO, registry *Registry) int {
	faultText, needle, envOut := lookupEnv(stdio.Env, EnvFault), lookupEnv(stdio.Env, EnvFilter), lookupEnv(stdio.Env, EnvOut)
	checkFilterRules(faultText, needle)
	fault, faultOp := parseFaultSpec(faultText)
	prepared := prepare(a.runConfig(fault, faultOp, needle), registry)
	outcome := execute(prepared, func(line string) { writeLine(stdio.Out, line) })
	if !outcomePassed(outcome) {
		return 1
	}
	if a.command == "check" {
		return compare(a.positional[0], outcome.Manifest, prepared.Corpus, stdio)
	}
	out, hasOut := a.flags["--out"]
	if !hasOut {
		out = envOut
	}
	if outcome.Manifest == nil {
		writeLine(stdio.Out, "# faulted or filtered run: no manifest written")
	} else if out != "" {
		need(os.MkdirAll(filepath.Dir(out), 0o755) == nil, "io", "cannot create the output directory")
		need(os.WriteFile(out, []byte(renderJSON(outcome.Manifest)), 0o644) == nil, "io", "cannot write the manifest")
		writeLine(stdio.Out, "# wrote "+out)
		sidecar := filepath.Join(filepath.Dir(out), LabelsFile)
		need(os.WriteFile(sidecar, []byte(renderJSON(labelsFor(prepared.Corpus, outcome.Manifest))), 0o644) == nil,
			"io", "cannot write the labels")
		writeLine(stdio.Out, "# wrote "+sidecar)
	}
	return 0
}

// baseName is the last element of a path written with either separator.
func baseName(path string) string {
	cut := strings.LastIndexAny(path, `\/`)
	return path[cut+1:]
}

// compare is `check`: the regenerated manifest must equal the committed one.
//
// toolchain.version is diagnostic (VER04i 8), so a different Go patch release is
// not drift; every other byte is.
func compare(committed string, fresh map[string]any, corpus *Corpus, stdio IO) int {
	name := baseName(committed)
	if fresh == nil {
		writeLine(stdio.Out, "not ok manifest: this run cannot produce a manifest (faulted or filtered)")
		return 1
	}
	text, err := readText(committed)
	if err != nil {
		writeLine(stdio.Out, "not ok manifest: cannot read the committed manifest "+name)
		return 1
	}
	old, current, ok := reparse(text, fresh)
	if !ok {
		writeLine(stdio.Out, "not ok manifest: the committed manifest is not a lane manifest")
		return 1
	}
	if renderJSON(current) != text {
		keys := map[string]bool{}
		for key := range current {
			keys[key] = true
		}
		for key := range old {
			keys[key] = true
		}
		var changed []string
		for key := range keys {
			if renderJSON(current[key]) != renderJSON(old[key]) {
				changed = append(changed, key)
			}
		}
		sort.Strings(changed)
		detail := strings.Join(changed, ", ")
		if detail == "" {
			detail = "formatting"
		}
		writeLine(stdio.Out, "not ok manifest: the committed manifest drifted ("+detail+")")
		return 1
	}
	writeLine(stdio.Out, "ok manifest "+name+" matches the regenerated manifest")
	expected := renderJSON(labelsFor(corpus, current))
	actual, err := readText(filepath.Join(filepath.Dir(committed), LabelsFile))
	if err != nil {
		writeLine(stdio.Out, "not ok "+LabelsFile+": missing or unreadable next to the manifest")
		return 1
	}
	if actual != expected {
		writeLine(stdio.Out, "not ok "+LabelsFile+": differs from the labels derived from the corpus")
		return 1
	}
	writeLine(stdio.Out, "ok "+LabelsFile+" matches the labels derived from the corpus")
	return 0
}

// readText reads a regular file as strict UTF-8 text, or reports failure.
func readText(path string) (text string, err error) {
	err = guard(func() { text = decodeUTF8(readBytes(path, maxTextBytes), "malformed-json") })
	return text, err
}

// reparse parses the committed manifest and builds the fresh manifest with the
// committed toolchain.version, so the two can be compared byte for byte.
func reparse(text string, fresh map[string]any) (old, current map[string]any, ok bool) {
	err := guard(func() {
		parsed, isObject := parseJSON([]byte(text)).(map[string]any)
		oldToolchain, hasToolchain := parsed["toolchain"].(map[string]any)
		if !isObject || !hasToolchain {
			fail("manifest", "not a manifest")
		}
		old = parsed
		current = map[string]any{}
		for key, value := range fresh {
			current[key] = value
		}
		current["toolchain"] = map[string]any{
			"name":    fresh["toolchain"].(map[string]any)["name"],
			"version": oldToolchain["version"],
		}
	})
	return old, current, err == nil
}

// childEnv is the environment of one sabotage child: a fault, a suite filter, no
// output file.
func childEnv(base []string, fault string) []string {
	var env []string
	for _, item := range base {
		key, _, _ := strings.Cut(item, "=")
		if key != EnvFault && key != EnvFilter && key != EnvOut {
			env = append(env, item)
		}
	}
	return append(env, EnvFault+"="+fault, EnvFilter+"=/exhaustive/")
}

// limitedBuffer keeps at most max bytes and discards the rest, so a child that
// prints without end cannot exhaust memory.
type limitedBuffer struct {
	buffer bytes.Buffer
	max    int
}

func (l *limitedBuffer) Write(p []byte) (int, error) {
	if room := l.max - l.buffer.Len(); room > 0 {
		l.buffer.Write(p[:min(room, len(p))])
	}
	return len(p), nil
}

func (a *args) commonFlags() []string {
	var out []string
	for _, flag := range commonFlag {
		out = append(out, flag, a.flags[flag])
	}
	return out
}

// sabotage re-runs `run` once per control with a fault set; each must fail loudly.
func sabotage(a *args, stdio IO) int {
	a.runConfig("", "", "") // refuse early if --binding/--package-dir are missing
	exe := stdio.Exe
	if exe == "" {
		self, err := os.Executable()
		need(err == nil, "io", "cannot find the running executable")
		exe = self
	}
	failures := 0
	for _, row := range listControlRows(a.flags["--corpus"]) {
		argv := append(append([]string(nil), stdio.ExeArgs...), "run")
		argv = append(argv, a.commonFlags()...)
		child := exec.Command(exe, argv...)
		child.Env = childEnv(stdio.Env, row)
		stdout := &limitedBuffer{max: 64 * 1024 * 1024}
		child.Stdout = stdout
		runErr := child.Run()
		status := 0
		var exit *exec.ExitError
		if errors.As(runErr, &exit) {
			status = exit.ExitCode()
		} else if runErr != nil {
			status = -1
		}
		caught := status == 1
		if caught {
			caught = false
			for _, line := range strings.Split(stdout.buffer.String(), "\n") {
				caught = caught || strings.HasPrefix(line, "not ok ")
			}
		}
		verdict := "not ok"
		if caught {
			verdict = "ok"
		} else {
			failures++
		}
		writeLine(stdio.Out, fmt.Sprintf("%s sabotage %s (exit %d)", verdict, row, status))
	}
	if failures > 0 {
		return 1
	}
	return 0
}

func validate(a *args, stdio IO) int {
	repoRoot, corpusDir := a.flags["--repo-root"], a.flags["--corpus"]
	relativeCorpus := requireInside(repoRoot, corpusDir, "corpus directory")
	corpus := loadCorpus(repoRoot, corpusDir)
	var registry []byte
	if registryPath, has := a.flags["--registry"]; has {
		registry = readBytes(registryPath, maxTextBytes)
	}
	manifestPath := a.positional[0]
	document := validateManifest(readBytes(manifestPath, maxTextBytes), corpus, relativeCorpus, registry, a.requireActive)
	sidecar := filepath.Join(filepath.Dir(manifestPath), LabelsFile)
	if _, err := os.Lstat(sidecar); err == nil {
		expected := renderJSON(labelsFor(corpus, document))
		if decodeUTF8(readBytes(sidecar, maxTextBytes), "malformed-json") != expected {
			usageFail("labels", "%s differs from the labels derived from the corpus", LabelsFile)
		}
	}
	writeLine(stdio.Out, "ok "+baseName(manifestPath)+" is a structurally valid lane manifest")
	return 0
}

// Main is the entry point: it runs the command in argv (without the program
// name) against registry and returns the process exit status.
func Main(argv []string, stdio IO, registry *Registry) int {
	status := 0
	err := guard(func() {
		parsed := parseArgs(argv)
		switch parsed.command {
		case "run", "check":
			status = runCommand(parsed, stdio, registry)
		case "sabotage":
			status = sabotage(parsed, stdio)
		case "list-faults":
			writeLine(stdio.Out, strings.Join(listControlRows(parsed.flags["--corpus"]), "\n"))
		case "source-digest":
			writeLine(stdio.Out, sourceDigest(parsed.positional[0]))
		case "validate-manifest":
			status = validate(parsed, stdio)
		}
	})
	if err != nil {
		// A LaneError is a refusal; anything else is a bug in the runner, not a
		// verdict about the library. Both exit 2: exit 1 is reserved for a failed
		// case.
		writeLine(stdio.Err, "error: "+err.Error())
		return 2
	}
	return status
}

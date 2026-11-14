package record

// Re-verifying `pre/` (KILN04 section 4.8, decision P11 and P12 step 1)
// ====================================================================
//
// `prepare` seals `pre/` before any BUILD runs. Every later command that uses a
// file of it (`kiln record step` now, `collect` in the next PR) must first prove
// that the directory is still what `prepare` wrote, because everything the
// recorder believes about the run (which actions exist, what each step is
// supposed to be, the sandbox every step runs in, the programs it may start)
// comes from there:
//
//	SEALED        sha256 and path of every other file, sorted, written last
//	binding.json  digests of the exports; the SHA-256 of THIS file is the
//	              "binding id" that names the whole shard
//
// The file modes (0444 and 0555) are cosmetic. A BUILD can not reach the record
// directory at all (the sandbox's new root does not contain it, and `prepare`
// refused every configuration in which it would), so what this check defends
// against is a bug, a stray tool, or an operator's mistake, and against a
// half-finished `prepare`, which has no SEALED file and so is refused here.
//
// What is checked, in order:
//
//	1  `--dir` is canonical; `pre/` is a real directory; SEALED is a regular
//	   file of a strict shape (one "<sha256>  <path>" line per file, sorted, so
//	   no path twice)
//	2  `pre/` holds exactly the files SEALED lists, SEALED itself, and the one
//	   directory `actions/`: nothing missing, nothing extra, no symlink
//	3  each file's SHA-256 equals the sealed one. The bytes that are hashed are
//	   the bytes that are then parsed (the file is read once), so a swap between
//	   "check" and "use" has no window to exist in
//	4  binding.json decodes strictly, and its digests of the graph, steps, host
//	   facts, inventory and sandbox configuration equal the sealed files'
//	5  tools.json, sandbox.json, classify.json and every action file decode
//	   strictly, and every action is one the sandbox configuration has an
//	   environment for
//
// `tree` (the tracked tree, tree.go) is hashed here like every other file, but
// decoded only when the classifier needs it (Pre.Tree): `step` verifies pre/ on
// every step, and decoding 394 thousand lines each time would cost more than it
// protects, while a file whose digest is right and which `prepare` decoded
// itself before sealing cannot have a malformed body.
//
// Nothing here consults PATH or the process environment.

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// Limits on what is read from `pre/`. They are generous (a real shard's graph
// is a few megabytes) and exist so a corrupt directory cannot exhaust memory.
const (
	maxSealedBytes   = 16 << 20  // SEALED itself
	maxSealedFile    = 1 << 30   // any file is hashed through this many bytes
	maxKeptFile      = 128 << 20 // a file whose bytes are kept and parsed
	maxPreEntries    = 1 << 20   // lines of SEALED
	sealedLineDigits = 64
)

// requiredPre are the files every sealed `pre/` holds, besides the actions.
var requiredPre = []string{
	"binding.json", "build-plan.json", "exec-wrapper.json", "graph.json", "host-facts.json",
	"inventory.json", "sandbox.json", "steps.json", "tools.json", "classify.json", "tree",
}

// Pre is a verified `pre/` directory.
type Pre struct {
	// Dir is the record directory and PreDir is `<Dir>/pre`.
	Dir, PreDir string
	// Binding is the decoded binding and BindingID the SHA-256 (lowercase hex)
	// of its file's bytes.
	Binding   recfmt.Binding
	BindingID string
	// Tools is tools.json and Config the parsed sandbox configuration;
	// ConfigPath is the file the sandbox is started with.
	Tools ToolsFile
	// Classify is classify.json: the host roots and pinned ecosystems the
	// classifier is given (decision P13).
	Classify   ClassifyFile
	Config     *sandbox.Config
	ConfigPath string
	// Sums is the digest of every file of `pre/` (relative path, slash
	// separated), exactly as SEALED lists it.
	Sums    map[string]string
	actions map[string]Action
}

// Action returns the planned action of that name.
func (p *Pre) Action(name string) (Action, bool) {
	a, ok := p.actions[name]
	return a, ok
}

// Actions lists the names of the planned actions, sorted.
func (p *Pre) Actions() []string {
	out := make([]string, 0, len(p.actions))
	for n := range p.actions {
		out = append(out, n)
	}
	sort.Strings(out)
	return out
}

// VerifyPre runs the checks above on `<dir>/pre`.
func VerifyPre(dir string) (*Pre, error) {
	if _, err := Canonical("the record directory", dir); err != nil {
		return nil, err
	}
	pre := filepath.Join(dir, "pre")
	if st, err := os.Lstat(pre); err != nil {
		return nil, fmt.Errorf("pre/: %v", err)
	} else if st.Mode()&os.ModeSymlink != 0 || !st.IsDir() {
		return nil, fmt.Errorf("pre/ is not a real directory")
	}
	sums, err := readSealed(pre)
	if err != nil {
		return nil, err
	}
	if err := checkPreContents(pre, sums); err != nil {
		return nil, err
	}
	// Hash every file once, keeping the bytes of the ones that are parsed.
	kept := map[string][]byte{}
	for _, rel := range sortedKeys(sums) {
		keep := rel == "binding.json" || rel == "tools.json" || rel == "sandbox.json" || rel == "steps.json" || rel == "classify.json" || strings.HasPrefix(rel, "actions/")
		data, sum, err := hashPreFile(filepath.Join(pre, filepath.FromSlash(rel)), keep)
		if err != nil {
			return nil, fmt.Errorf("pre/%s: %w", rel, err)
		}
		if sum != sums[rel] {
			return nil, fmt.Errorf("pre/%s was modified: its digest is %s, SEALED says %s", rel, sum[:12], sums[rel][:12])
		}
		if keep {
			kept[rel] = data
		}
	}
	p := &Pre{Dir: dir, PreDir: pre, Sums: sums, ConfigPath: filepath.Join(pre, "sandbox.json"), BindingID: sums["binding.json"], actions: map[string]Action{}}
	if p.Binding, err = recfmt.DecodeBinding(kept["binding.json"]); err != nil {
		return nil, err
	}
	for _, d := range []struct{ rel, got string }{
		{"graph.json", p.Binding.GraphSHA256}, {"steps.json", p.Binding.StepsSHA256}, {"host-facts.json", p.Binding.HostFactsSHA256},
		{"inventory.json", p.Binding.InventorySHA256}, {"sandbox.json", p.Binding.SandboxConfigSHA256},
	} {
		if d.got != sums[d.rel] {
			return nil, fmt.Errorf("binding.json names a different %s than the one sealed", d.rel)
		}
	}
	if p.Tools, err = decodeTools(kept["tools.json"]); err != nil {
		return nil, err
	}
	if p.Classify, err = DecodeClassify(kept["classify.json"]); err != nil {
		return nil, fmt.Errorf("pre/classify.json: %w", err)
	}
	if p.Config, err = sandbox.ParseConfig(kept["sandbox.json"]); err != nil {
		return nil, fmt.Errorf("pre/sandbox.json: %w", err)
	}
	steps, err := legacy.DecodeSteps(kept["steps.json"])
	if err != nil {
		return nil, fmt.Errorf("pre/steps.json: %w", err)
	}
	for rel, data := range kept {
		if !strings.HasPrefix(rel, "actions/") {
			continue
		}
		a, err := DecodeAction(data)
		if err != nil {
			return nil, fmt.Errorf("pre/%s: %w", rel, err)
		}
		if ActionFile(a.Action) != rel {
			return nil, fmt.Errorf("pre/%s holds the action %q, whose file is %s", rel, printable(a.Action), ActionFile(a.Action))
		}
		if _, err := p.Config.EnvFor(a.Action); err != nil {
			return nil, fmt.Errorf("pre/%s: the sandbox configuration has no environment for %q", rel, printable(a.Action))
		}
		// The plan an action file holds is the sealed steps export's, line for
		// line: the two were written together, and a mismatch means one of them
		// was replaced.
		if err := stepsMatch(a, steps); err != nil {
			return nil, fmt.Errorf("pre/%s: %w", rel, err)
		}
		p.actions[a.Action] = a
	}
	return p, nil
}

// ReadSealed reads one sealed file again, hashing the very bytes it returns and
// requiring the digest SEALED lists, so a file swapped after VerifyPre is
// refused. The classifier uses it for the files VerifyPre only hashed (the
// graph, the host facts, the tree).
func (p *Pre) ReadSealed(rel string) ([]byte, error) {
	want, ok := p.Sums[rel]
	if !ok {
		return nil, fmt.Errorf("pre/%s is not a sealed file", printable(rel))
	}
	data, sum, err := hashPreFile(filepath.Join(p.PreDir, filepath.FromSlash(rel)), true)
	if err != nil {
		return nil, fmt.Errorf("pre/%s: %w", rel, err)
	}
	if sum != want {
		return nil, fmt.Errorf("pre/%s was modified: its digest is %s, SEALED says %s", rel, sum[:12], want[:12])
	}
	return data, nil
}

// Tree reads and strictly decodes the sealed tracked tree (pre/tree).
func (p *Pre) Tree() (*TreeFile, error) {
	data, err := p.ReadSealed("tree")
	if err != nil {
		return nil, err
	}
	t, err := DecodeTree(data)
	if err != nil {
		return nil, fmt.Errorf("pre/tree: %w", err)
	}
	return t, nil
}

// readSealed reads and parses SEALED: "<64 hex>  <path>\n" per line, strictly
// ascending by path (which also means no path twice).
func readSealed(pre string) (map[string]string, error) {
	f, err := openNoFollow(filepath.Join(pre, "SEALED"))
	if err != nil {
		return nil, fmt.Errorf("pre/SEALED: %v (a record directory that was not sealed is never used)", err)
	}
	defer f.Close()
	st, err := f.Stat()
	if err != nil {
		return nil, err
	}
	if !st.Mode().IsRegular() {
		return nil, fmt.Errorf("pre/SEALED is not a regular file")
	}
	data, err := io.ReadAll(io.LimitReader(f, maxSealedBytes+1))
	if err != nil {
		return nil, err
	}
	if len(data) > maxSealedBytes {
		return nil, fmt.Errorf("pre/SEALED exceeds %d bytes", maxSealedBytes)
	}
	if len(data) == 0 || data[len(data)-1] != '\n' {
		return nil, fmt.Errorf("pre/SEALED does not end in a newline")
	}
	sums := map[string]string{}
	prev := ""
	for i, line := range strings.Split(strings.TrimSuffix(string(data), "\n"), "\n") {
		if i >= maxPreEntries {
			return nil, fmt.Errorf("pre/SEALED has more than %d entries", maxPreEntries)
		}
		if len(line) < sealedLineDigits+3 || line[sealedLineDigits:sealedLineDigits+2] != "  " || !hexDigest(line[:sealedLineDigits]) {
			return nil, fmt.Errorf("pre/SEALED line %d is not \"<sha256>  <path>\"", i+1)
		}
		digest, rel := line[:sealedLineDigits], line[sealedLineDigits+2:]
		if err := checkSealedPath(rel); err != nil {
			return nil, fmt.Errorf("pre/SEALED line %d: %v", i+1, err)
		}
		if i > 0 && rel <= prev {
			return nil, fmt.Errorf("pre/SEALED line %d: %q is not after %q (the list is sorted, each path once)", i+1, printable(rel), printable(prev))
		}
		prev = rel
		sums[rel] = digest
	}
	for _, rel := range requiredPre {
		if _, ok := sums[rel]; !ok {
			return nil, fmt.Errorf("pre/SEALED does not list %s", rel)
		}
	}
	return sums, nil
}

// checkSealedPath requires a path SEALED may list: relative, clean, slash
// separated, no control characters, either a top-level file or one file of
// actions/. SEALED never lists itself.
func checkSealedPath(rel string) error {
	if rel == "" || rel == "SEALED" || path.IsAbs(rel) || path.Clean(rel) != rel || strings.ContainsAny(rel, "\\\x00") {
		return fmt.Errorf("%q is not a file of pre/", printable(rel))
	}
	for _, r := range rel {
		if r < 0x20 || r == 0x7f {
			return fmt.Errorf("%q holds a control character", printable(rel))
		}
	}
	if dir, base := path.Split(rel); dir != "" && (dir != "actions/" || base == "") {
		return fmt.Errorf("%q is not directly in pre/ or pre/actions/", printable(rel))
	}
	return nil
}

// checkPreContents requires pre/ to hold exactly the sealed files, SEALED, and
// the directory actions/.
func checkPreContents(pre string, sums map[string]string) error {
	seen := 0
	err := filepath.WalkDir(pre, func(p string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, _ := filepath.Rel(pre, p)
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
		case rel == "SEALED":
		default:
			if _, ok := sums[rel]; !ok {
				return fmt.Errorf("pre/%s is not listed in SEALED", rel)
			}
			seen++
		}
		return nil
	})
	if err != nil {
		return err
	}
	if seen != len(sums) {
		for _, rel := range sortedKeys(sums) {
			if _, err := os.Lstat(filepath.Join(pre, filepath.FromSlash(rel))); err != nil {
				return fmt.Errorf("pre/%s is listed in SEALED but missing", rel)
			}
		}
		return fmt.Errorf("pre/ does not hold the files SEALED lists")
	}
	return nil
}

// hashPreFile reads one file of pre/ once: through a non-following handle, a
// regular file, hashing every byte and keeping them when keep is set.
func hashPreFile(p string, keep bool) ([]byte, string, error) {
	f, err := openNoFollow(p)
	if err != nil {
		return nil, "", err
	}
	defer f.Close()
	st, err := f.Stat()
	if err != nil {
		return nil, "", err
	}
	if !st.Mode().IsRegular() {
		return nil, "", fmt.Errorf("not a regular file")
	}
	h := sha256.New()
	var buf bytes.Buffer
	var w io.Writer = h
	limit := int64(maxSealedFile)
	if keep {
		w, limit = io.MultiWriter(h, &buf), maxKeptFile
	}
	n, err := io.Copy(w, io.LimitReader(f, limit+1))
	if err != nil {
		return nil, "", err
	}
	if n > limit {
		return nil, "", fmt.Errorf("exceeds %d bytes", limit)
	}
	return buf.Bytes(), hex.EncodeToString(h.Sum(nil)), nil
}

func sortedKeys(m map[string]string) []string {
	out := make([]string, 0, len(m))
	for k := range m {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}

// decodeTools reads tools.json strictly: no unknown key, no trailing data, the
// right schema, absolute clean paths, and an environment whose entries all
// have the form NAME=value with no name twice.
func decodeTools(data []byte) (ToolsFile, error) {
	var t ToolsFile
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&t); err != nil {
		return ToolsFile{}, fmt.Errorf("tools: %w", err)
	}
	if _, err := dec.Token(); err != io.EOF {
		return ToolsFile{}, fmt.Errorf("tools: unexpected data after the JSON value")
	}
	if t.Schema != ToolsSchema {
		return ToolsFile{}, fmt.Errorf("tools: schema %q", printable(t.Schema))
	}
	for _, p := range []struct{ what, v string }{{"git", t.Git}, {"strace", t.Strace}} {
		if !filepath.IsAbs(p.v) || filepath.Clean(p.v) != p.v {
			return ToolsFile{}, fmt.Errorf("tools: %s %q is not an absolute, clean path", p.what, printable(p.v))
		}
	}
	if _, err := envNames(t.BuildToolEnv); err != nil {
		return ToolsFile{}, fmt.Errorf("tools: build_tool_env: %w", err)
	}
	return t, nil
}

// envNames maps the NAME of every NAME=value entry to the whole entry, refusing
// an entry without a name or an '=', and a name that appears twice (execve(2)
// would let one of them win silently, and a comparison of environments must
// not paper over that). An error never carries a value: an environment may
// hold a secret.
func envNames(env []string) (map[string]string, error) {
	out := make(map[string]string, len(env))
	for _, e := range env {
		i := strings.IndexByte(e, '=')
		if i <= 0 {
			return nil, fmt.Errorf("an entry is not NAME=value")
		}
		name := e[:i]
		if _, dup := out[name]; dup {
			return nil, fmt.Errorf("the name %q appears twice", printable(name))
		}
		out[name] = e
	}
	return out, nil
}

package warrantcorpus

// Shared test scaffolding: a reference library, bindings and tampered corpora.
//
// The runner must work with ANY Go library, so these tests do not import
// ct-compare (the runner has no dependency on a leaf library). Instead the
// package's own tests define a small reference library (RefEq, RefSelect, ...)
// and a few deliberately wrong ones, and bind them through a Registry. The real
// corpus (emitted by the Rust compiler) is used unmodified wherever possible: its
// digests come from outside this package, which is what makes agreeing with them
// meaningful.
//
// Every fixture is copied as BYTES. Reading a file as text and writing it back is
// a newline-translating round trip on some platforms; bytes are not.

import (
	"errors"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

const importPath = "github.com/adhithyan15/coding-adventures/code/packages/go/warrant-corpus"

// ErrRefLength is the sentinel of the reference library.
var ErrRefLength = errors.New("reference: length mismatch")

// The reference library. Exported-looking names because the binding grammar
// names exports; each is a top-level function so the runtime can name it.

func RefEq(a, b []byte) bool { return string(a) == string(b) }

func RefEqU64(a, b uint64) bool { return a == b }

func RefSelect(a, b []byte, choice bool) ([]byte, error) {
	if len(a) != len(b) {
		return nil, ErrRefLength
	}
	source := b
	if choice {
		source = a
	}
	return append([]byte(nil), source...), nil
}

// Deliberately wrong libraries, one defect each.

func RefEqAlwaysTrue(a, b []byte) bool { return true }

func RefEqU64Low32(a, b uint64) bool { return uint32(a) == uint32(b) }

func RefSelectAlias(a, b []byte, choice bool) ([]byte, error) {
	if len(a) != len(b) {
		return nil, ErrRefLength
	}
	if choice {
		return a, nil // shares memory with its argument
	}
	return b, nil
}

func RefSelectNoError(a, b []byte, choice bool) ([]byte, error) {
	if len(a) != len(b) {
		return []byte{}, nil // returns a value where the model expects Err
	}
	return RefSelect(a, b, choice)
}

func RefSelectOther(a, b []byte, choice bool) ([]byte, error) {
	if len(a) != len(b) {
		return nil, errors.New("reference: some other failure")
	}
	return RefSelect(a, b, choice)
}

func RefSelectPanics(a, b []byte, choice bool) ([]byte, error) {
	if len(a) != len(b) {
		panic("boom")
	}
	return RefSelect(a, b, choice)
}

func RefSelectWrapped(a, b []byte, choice bool) ([]byte, error) {
	if len(a) != len(b) {
		return nil, errors.Join(ErrRefLength, errors.New("wrapped"))
	}
	return RefSelect(a, b, choice)
}

// Wrong adapters bind to library functions of their own name, so provenance holds.
func RefEqReturnsInt(a, b []byte) bool  { return true }
func RefEqReturnsNil(a, b []byte) bool  { return true }
func RefEqReturnsPair(a, b []byte) bool { return true }

// RefWidth takes the width as a trailing argument (width=runtime).
func RefWidth(a, b []byte, width int) bool { return len(a) == width && string(a) == string(b) }

// bytesArgs unpacks two Bytes arguments.
func bytesArgs(args []any) ([]byte, []byte) { return args[0].([]byte), args[1].([]byte) }

// refRegistry is the registry of the correct reference library plus every wrong
// variant, keyed by export name.
func refRegistry() *Registry {
	eq := func(args []any) (any, error) { a, b := bytesArgs(args); return RefEq(a, b), nil }
	eqU64 := func(args []any) (any, error) { return RefEqU64(args[0].(uint64), args[1].(uint64)), nil }
	selection := func(f func(a, b []byte, choice bool) ([]byte, error)) func([]any) (any, error) {
		return func(args []any) (any, error) {
			a, b := bytesArgs(args)
			out, err := f(a, b, args[2].(bool))
			if err != nil {
				return nil, err
			}
			return out, nil
		}
	}
	return &Registry{
		Exports: map[string]Export{
			"RefEq":            {Call: eq, Impl: RefEq},
			"RefEqU64":         {Call: eqU64, Impl: RefEqU64},
			"RefSelect":        {Call: selection(RefSelect), Impl: RefSelect},
			"RefEqAlwaysTrue":  {Call: func(args []any) (any, error) { a, b := bytesArgs(args); return RefEqAlwaysTrue(a, b), nil }, Impl: RefEqAlwaysTrue},
			"RefEqU64Low32":    {Call: func(args []any) (any, error) { return RefEqU64Low32(args[0].(uint64), args[1].(uint64)), nil }, Impl: RefEqU64Low32},
			"RefSelectAlias":   {Call: selection(RefSelectAlias), Impl: RefSelectAlias},
			"RefSelectNoError": {Call: selection(RefSelectNoError), Impl: RefSelectNoError},
			"RefSelectOther":   {Call: selection(RefSelectOther), Impl: RefSelectOther},
			"RefSelectPanics":  {Call: selection(RefSelectPanics), Impl: RefSelectPanics},
			"RefSelectWrapped": {Call: selection(RefSelectWrapped), Impl: RefSelectWrapped},
			"RefWidth": {Call: func(args []any) (any, error) {
				a, b := bytesArgs(args)
				return RefWidth(a, b, args[2].(int)), nil
			}, Impl: RefWidth},
			// Wrong adapters over a correct library function.
			"RefEqReturnsInt":  {Call: func(args []any) (any, error) { return 1, nil }, Impl: RefEqReturnsInt},
			"RefEqReturnsNil":  {Call: func(args []any) (any, error) { return nil, nil }, Impl: RefEqReturnsNil},
			"RefEqReturnsPair": {Call: func(args []any) (any, error) { return Pair{"err", "X"}, nil }, Impl: RefEqReturnsPair},
		},
		Errors: map[string]error{"ErrRefLength": ErrRefLength, "ErrNil": nil},
	}
}

// bindingText is the reference binding, with ops optionally replaced.
func bindingText(replace map[string]string) string {
	ops := []string{
		"op eq export=RefEq",
		"op eq_fixed export=RefEq width=implicit",
		"op select_bytes export=RefSelect errors=CT_SELECT_LENGTH_MISMATCH:ErrRefLength",
		"op eq_u64 export=RefEqU64",
	}
	for i, line := range ops {
		name := strings.Fields(line)[1]
		if replacement, has := replace[name]; has {
			ops[i] = replacement
		}
	}
	return "lane go\npackage go/ct-compare\nmodule ct_compare\nimport " + importPath +
		"\nsource .\nseed 0x5741_5252_414e_5421\nvalues Bool=bool Bytes=[]byte Nat=uint64 U64=uint64\n" +
		strings.Join(ops, "\n") + "\n"
}

var (
	realRepoRoot string
	realCorpus   string
)

func init() {
	_, file, _, _ := runtime.Caller(0)
	realRepoRoot = filepath.Clean(filepath.Join(filepath.Dir(file), "..", "..", "..", ".."))
	realCorpus = filepath.Join(realRepoRoot, "code", "specs", "fixtures", "ct-compare-warrant-v1")
}

// fixture is a scratch repository: a copy of the real corpus and spec, a package
// directory holding a dummy source file, and a binding.
type fixture struct {
	root, corpusDir, pkgDir, binding string
}

type fixtureOptions struct {
	corpus  func(document map[string]any)
	tfv     func(text string) string
	spec    func(data []byte) []byte
	binding string
	mini    bool
}

func mustRead(t *testing.T, path string) []byte {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func mustWrite(t *testing.T, path string, data []byte) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, data, 0o644); err != nil {
		t.Fatal(err)
	}
}

var keepClaims = []string{"eq_fixed_agrees_w2", "eq_fixed_agrees_w4"}

// miniTweak reduces the corpus to a consistent two-suite slice (512 cases): whole
// corpus runs take a moment, so tests of plumbing use this slice and one test runs
// the whole real corpus.
func miniTweak(document map[string]any) {
	var suites []any
	for _, suite := range document["suites"].([]any) {
		if containsString(keepClaims, suite.(map[string]any)["claim"].(string)) {
			suites = append(suites, suite)
		}
	}
	document["suites"] = suites
	filter := func(key string) {
		var kept []any
		for _, row := range document[key].([]any) {
			if row.(map[string]any)["op"] == "eq_fixed" {
				kept = append(kept, row)
			}
		}
		document[key] = kept
	}
	filter("examples")
	filter("faults")
	filter("controls")
	var required []any
	for _, claim := range keepClaims {
		required = append(required, "warrant/ct_compare/exhaustive/"+claim)
	}
	for _, control := range document["controls"].([]any) {
		control.(map[string]any)["required_claims"] = required
	}
}

func miniTFV(text string) string {
	lines := strings.Split(text, "\n")
	kept := []string{lines[0]}
	for _, line := range lines[1:] {
		if strings.Contains(line, "Kop S65715f6669786564") {
			kept = append(kept, line)
		}
	}
	return strings.Join(kept, "\n") + "\n"
}

// newFixture builds a scratch repository. The library's "source file" is the
// dummy pkg/lib.go, via the provenance seam, so these tests do not depend on where
// this package's real sources live.
func newFixture(t *testing.T, options fixtureOptions) *fixture {
	t.Helper()
	root := t.TempDir()
	if resolved, err := filepath.EvalSymlinks(root); err == nil {
		root = resolved
	}
	f := &fixture{
		root:      root,
		corpusDir: filepath.Join(root, "code", "specs", "fixtures", "ct-compare-warrant-v1"),
		pkgDir:    filepath.Join(root, "code", "packages", "go", "ct-compare"),
	}
	f.binding = filepath.Join(f.pkgDir, "evidence.bindings")
	document := parseJSON(mustRead(t, filepath.Join(realCorpus, "corpus.json"))).(map[string]any)
	tfv := string(mustRead(t, filepath.Join(realCorpus, "cases.tfv")))
	if options.mini {
		miniTweak(document)
		tfv = miniTFV(tfv)
	}
	if options.corpus != nil {
		options.corpus(document)
	}
	if options.tfv != nil {
		tfv = options.tfv(tfv)
	}
	spec := mustRead(t, filepath.Join(realRepoRoot, "code", "specs", "ct-compare.warrant"))
	if options.spec != nil {
		spec = options.spec(spec)
	}
	mustWrite(t, filepath.Join(f.corpusDir, "corpus.json"), []byte(renderJSON(document)))
	mustWrite(t, filepath.Join(f.corpusDir, "cases.tfv"), []byte(tfv))
	mustWrite(t, filepath.Join(f.corpusDir, "consumers.json"), mustRead(t, filepath.Join(realCorpus, "consumers.json")))
	mustWrite(t, filepath.Join(root, "code", "specs", "ct-compare.warrant"), spec)
	mustWrite(t, filepath.Join(f.pkgDir, "lib.go"), []byte("package lib\n"))
	text := options.binding
	if text == "" {
		text = bindingText(nil)
	}
	mustWrite(t, f.binding, []byte(text))

	// The provenance seam: keep the runtime's real symbol name, but report the
	// fixture's own source file.
	previous := symbolOf
	symbolOf = func(impl any) (string, string, bool) {
		name, _, ok := funcSymbol(impl)
		return name, filepath.Join(f.pkgDir, "lib.go"), ok
	}
	t.Cleanup(func() { symbolOf = previous })
	return f
}

func (f *fixture) config() RunConfig {
	return RunConfig{RepoRoot: f.root, CorpusDir: f.corpusDir, BindingPath: f.binding, PackageDir: f.pkgDir}
}

func (f *fixture) args(command string, extra ...string) []string {
	return append([]string{command, "--repo-root", f.root, "--corpus", f.corpusDir, "--binding", f.binding, "--package-dir", f.pkgDir}, extra...)
}

// laneCode runs f and returns the code of the *LaneError it panics with ("" when
// it does not panic).
func laneCode(t *testing.T, f func()) string {
	t.Helper()
	err := guard(f)
	if err == nil {
		return ""
	}
	var lane *LaneError
	if !errors.As(err, &lane) {
		t.Fatalf("not a lane error: %v", err)
	}
	return lane.Code
}

func wantCode(t *testing.T, want string, f func()) {
	t.Helper()
	if got := laneCode(t, f); got != want {
		t.Fatalf("lane error code = %q, want %q", got, want)
	}
}

// collect gathers emitted lines.
func collect() (*[]string, func(string)) {
	lines := &[]string{}
	return lines, func(line string) { *lines = append(*lines, line) }
}

func hasLine(lines []string, prefix string) bool {
	for _, line := range lines {
		if strings.HasPrefix(line, prefix) {
			return true
		}
	}
	return false
}

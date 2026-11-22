package warrantcorpus

import (
	"embed"
	"encoding/binary"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"unicode/utf8"
)

// # The warrant-lane-evidence-v1 manifest: build, digest, validate
//
// A manifest is the lane's RECEIPT: what ran, against which corpus, binding and
// source, and what happened. Every number in a report is meant to be read from
// it, never typed by hand (VER04i section 0).
//
// # warrant-src-v1 -- digesting a source tree
//
// source_sha256 must change whenever code that can change the library's behaviour
// changes, and never because of anything else. The framing is the same one Rust,
// Python and TypeScript use, so every lane's producers and the checker agree:
//
//	"warrant-src-v1\0"
//	u64-LE file count
//	for each file, sorted by path bytes:
//	    u64-LE path length,    path bytes   (relative to the source root, "/")
//	    u64-LE content length, content bytes
//
// Every field is length-prefixed, so moving bytes between a path and its content,
// or between two files, changes a length and therefore the digest. For Go the
// files are every `*.go` beneath the root except `*_test.go` (VER04i 5.3, 8.1),
// skipping `vendor`, `testdata` and directories whose name starts with `.` or `_`
// (the directories the go tool itself ignores). A symlink, a non-UTF-8 name, an
// oversized file or an over-deep tree is an error: the digest must cover exactly
// what the compiler would read, so anything it cannot account for fails closed.
const (
	maxSourceFileBytes = 8 * 1024 * 1024
	maxSourceDepth     = 32
	// ManifestSchema is the schema string of a lane manifest.
	ManifestSchema = "warrant-lane-evidence-v1"
	runnerName     = "warrant-corpus"
)

var (
	packageNameRE = regexp.MustCompile(`^[a-z0-9][a-z0-9_-]*$`)
	seedDecimalRE = regexp.MustCompile(`^(?:0|[1-9][0-9]*)$`)
)

// runnerSources holds the runner's own source, as compiled. Embedding it means
// runner.src_sha256 digests the bytes that actually ran, wherever the binary is
// started from.
//
//go:embed *.go
var runnerSources embed.FS

// SourceFile is a source file: its path relative to the root, and its bytes.
type SourceFile struct {
	Path string
	Data []byte
}

func isSourceName(name string) bool {
	return strings.HasSuffix(name, ".go") && !strings.HasSuffix(name, "_test.go")
}

func skippedDirectory(name string) bool {
	return name == "vendor" || name == "testdata" || strings.HasPrefix(name, ".") || strings.HasPrefix(name, "_")
}

// readSourceTree reads every production source file under root.
func readSourceTree(root string) []SourceFile {
	var files []SourceFile
	var walk func(directory, relative string, depth int)
	walk = func(directory, relative string, depth int) {
		need(depth <= maxSourceDepth, "source-tree", "source tree is nested too deeply")
		entries, err := os.ReadDir(directory)
		need(err == nil, "source-tree", "cannot list the source tree")
		for _, entry := range entries { // ReadDir sorts by file name
			name := entry.Name()
			need(utf8.ValidString(name), "source-tree", "non-UTF-8 file name")
			rel := name
			if relative != "" {
				rel = relative + "/" + name
			}
			full := filepath.Join(directory, name)
			info, err := os.Lstat(full)
			need(err == nil, "source-tree", "cannot stat %s", rel)
			need(info.Mode()&os.ModeSymlink == 0, "source-tree", "%s is a symlink", rel)
			switch {
			case info.IsDir():
				if !skippedDirectory(name) {
					walk(full, rel, depth+1)
				}
			case info.Mode().IsRegular() && isSourceName(name):
				need(info.Size() <= maxSourceFileBytes, "source-tree", "%s is larger than the limit", rel)
				data, err := os.ReadFile(full)
				need(err == nil, "source-tree", "cannot read %s", rel)
				files = append(files, SourceFile{rel, data})
			}
		}
	}
	walk(root, "", 0)
	sort.Slice(files, func(i, j int) bool { return files[i].Path < files[j].Path })
	return files
}

func appendU64(out []byte, n int) []byte { return binary.LittleEndian.AppendUint64(out, uint64(n)) }

// frameSources is the warrant-src-v1 byte framing of files (see the file comment).
func frameSources(files []SourceFile) []byte {
	ordered := append([]SourceFile(nil), files...)
	sort.Slice(ordered, func(i, j int) bool { return ordered[i].Path < ordered[j].Path })
	out := []byte("warrant-src-v1\x00")
	out = appendU64(out, len(ordered))
	for _, file := range ordered {
		out = appendU64(out, len(file.Path))
		out = append(out, file.Path...)
		out = appendU64(out, len(file.Data))
		out = append(out, file.Data...)
	}
	return out
}

// sourceDigest is source_sha256 of the source tree at root.
func sourceDigest(root string) string {
	info, err := os.Stat(root)
	need(err == nil && info.IsDir(), "source-tree", "source root is not a directory")
	return sha256Hex(frameSources(readSourceTree(root)))
}

// runnerDigest is runner.src_sha256: the digest of this package's own non-test
// source, as embedded at compile time.
func runnerDigest() string {
	entries, err := runnerSources.ReadDir(".")
	need(err == nil, "source-tree", "cannot read the embedded runner source")
	var files []SourceFile
	for _, entry := range entries {
		if !isSourceName(entry.Name()) {
			continue
		}
		data, err := runnerSources.ReadFile(entry.Name())
		need(err == nil, "source-tree", "cannot read the embedded runner source")
		files = append(files, SourceFile{entry.Name(), data})
	}
	return sha256Hex(frameSources(files))
}

// toolchainVersion is the Go release that built this binary, without the "go"
// prefix (diagnostic only; `check` ignores it).
func toolchainVersion() string { return strings.TrimPrefix(runtime.Version(), "go") }

// buildManifest assembles the manifest for a clean, passing run.
func buildManifest(corpus *Corpus, binding *Binding, corpusRelative, sourceRoot string, rows []map[string]any) map[string]any {
	tests := append([]map[string]any(nil), rows...)
	sort.Slice(tests, func(i, j int) bool { return tests[i]["id"].(string) < tests[j]["id"].(string) })
	return map[string]any{
		"schema":         ManifestSchema,
		"lane":           binding.Lane,
		"package":        binding.Package,
		"module":         binding.Module,
		"spec":           corpus.Spec,
		"spec_sha256":    corpus.SpecSha256,
		"corpus":         corpusRelative,
		"corpus_sha256":  corpus.CorpusSha256,
		"cases_sha256":   corpus.CasesSha256,
		"runner":         map[string]any{"name": runnerName, "lane": binding.Lane, "src_sha256": runnerDigest()},
		"binding_sha256": sha256Hex(binding.Raw),
		"source_sha256":  sourceDigest(sourceRoot),
		"toolchain":      map[string]any{"name": "go", "version": toolchainVersion()},
		"evidence_seed":  strconv.FormatUint(binding.Seed, 10),
		"faulted":        false,
		"tests":          tests,
	}
}

// ---------------------------------------------------------------------------
// Structural validation (a Go port of WD-004a's closed ct-compare-v0 rules)
// ---------------------------------------------------------------------------

var (
	manifestKeys = []string{"schema", "lane", "package", "module", "spec", "spec_sha256", "corpus",
		"corpus_sha256", "cases_sha256", "runner", "binding_sha256", "source_sha256",
		"toolchain", "evidence_seed", "faulted", "tests"}
	rowKeys = map[string][]string{
		"example": {"id", "layer", "kind", "op", "cases", "verdict"},
		"exhaustive": {"id", "layer", "kind", "op", "claim", "cases", "discarded", "calls",
			"inputs_sha256", "expected_sha256", "domain", "verdict"},
		"negctl": {"id", "layer", "kind", "fault", "op", "target", "caught_by", "verdict"},
	}
)

func isDigest(value any, where string) {
	s, ok := value.(string)
	need(ok && hex64RE.MatchString(s), "manifest", "%s must be a lowercase SHA-256", where)
}

func manifestObject(value any, keys []string, where string) map[string]any {
	object, ok := value.(map[string]any)
	valid := ok && len(object) == len(keys)
	if valid {
		for _, key := range keys {
			if _, has := object[key]; !has {
				valid = false
			}
		}
	}
	need(valid, "manifest", "%s has unexpected fields", where)
	return object
}

// validateManifest validates a lane manifest against the corpus it claims to have
// run.
//
// This mirrors ca-assurance-report's validate_lane_evidence (WD-004a) and the
// Python and TypeScript lanes' ports of it: closed key sets, exact ids and counts,
// sorted rows, and joins on digests and controls. It checks STRUCTURE AND JOINS
// ONLY. It does not run anything, and it cannot tell a real run from a
// hand-written file that happens to be consistent -- that is what regenerating
// (`check`) is for.
func validateManifest(manifestBytes []byte, corpus *Corpus, corpusRelative string, registryBytes []byte, requireActive bool) map[string]any {
	parsed := parseJSON(manifestBytes)
	// `pycache_isolated` is the one lane-specific key (VER04i 8.1): required, and
	// exactly true, for the Python lane; forbidden for every other lane. The lane
	// is read before the closed key check because the key set depends on it.
	keys := manifestKeys
	if object, isObject := parsed.(map[string]any); isObject {
		_, has := object["pycache_isolated"]
		if object["lane"] == "python" {
			need(has, "manifest", "pycache_isolated is required for the Python lane (VER04i 8.1)")
			keys = append(append([]string{}, manifestKeys...), "pycache_isolated")
		} else {
			need(!has, "manifest", "pycache_isolated is only recorded by the Python lane")
		}
	}
	root := manifestObject(parsed, keys, "manifest")
	if _, has := root["pycache_isolated"]; has {
		need(root["pycache_isolated"] == true, "manifest", "pycache_isolated must be true for the Python lane")
	}
	need(root["schema"] == ManifestSchema, "manifest", "unsupported schema")
	lane, laneIsString := root["lane"].(string)
	need(laneIsString && identRE.MatchString(lane), "manifest", "bad lane")
	packageName, _ := root["package"].(string)
	parts := strings.Split(packageName, "/")
	need(len(parts) == 2 && parts[0] == lane && packageNameRE.MatchString(parts[1]),
		"manifest", "package must be <lane>/<name> and match the lane")
	need(root["module"] == corpus.Module, "manifest", "module differs from the corpus")
	need(root["spec"] == corpus.Spec && root["spec_sha256"] == corpus.SpecSha256,
		"manifest", "spec path or digest differs from the corpus")
	need(root["corpus"] == corpusRelative, "manifest", "corpus path differs")
	need(root["corpus_sha256"] == corpus.CorpusSha256 && root["cases_sha256"] == corpus.CasesSha256,
		"manifest", "corpus or cases digest differs from the files")
	runnerRow := manifestObject(root["runner"], []string{"name", "lane", "src_sha256"}, "runner")
	need(runnerRow["name"] == runnerName && runnerRow["lane"] == lane, "manifest", "bad runner")
	isDigest(runnerRow["src_sha256"], "runner.src_sha256")
	isDigest(root["binding_sha256"], "binding_sha256")
	isDigest(root["source_sha256"], "source_sha256")
	toolchain := manifestObject(root["toolchain"], []string{"name", "version"}, "toolchain")
	_, nameOK := toolchain["name"].(string)
	_, versionOK := toolchain["version"].(string)
	need(nameOK && versionOK, "manifest", "bad toolchain")
	seed, seedIsString := root["evidence_seed"].(string)
	if seedIsString && seedDecimalRE.MatchString(seed) {
		_, err := strconv.ParseUint(seed, 10, 64)
		seedIsString = err == nil
	} else {
		seedIsString = false
	}
	need(seedIsString, "manifest", "evidence_seed must be a canonical u64 decimal string")
	need(root["faulted"] == false, "manifest", "faulted manifests are not accepted")

	rows, isList := root["tests"].([]any)
	expectedRows := len(corpus.Examples) + len(corpus.Suites) + len(corpus.Controls)
	need(isList && len(rows) == expectedRows, "manifest", "wrong number of rows")
	ids := make([]string, len(rows))
	byID := map[string]any{}
	for i, row := range rows {
		fields, isObj := row.(map[string]any)
		id := ""
		if isObj {
			id, _ = fields["id"].(string)
		}
		need(isObj && id != "", "manifest", "row without an id")
		_, duplicate := byID[id]
		need(!duplicate, "manifest", "duplicate test id")
		byID[id] = row
		ids[i] = id
	}
	for i := 1; i < len(ids); i++ {
		need(ids[i-1] < ids[i], "manifest", "rows must be sorted by id")
	}
	joinRows(byID, corpus)
	if registryBytes != nil {
		joinRegistry(registryBytes, lane, packageName, root["module"].(string), requireActive)
	}
	return root
}

// jsonValue normalises Go ints so they compare equal to parsed JSON integers.
func jsonValue(value any) any {
	if n, ok := value.(int); ok {
		return int64(n)
	}
	return value
}

// agrees: a row is a portable, passing `kind` row whose fields equal expected.
func agrees(row map[string]any, kind string, expected map[string]any) {
	all := map[string]any{"kind": kind, "layer": "portable", "verdict": "pass"}
	for key, value := range expected {
		all[key] = value
	}
	for key, value := range all {
		need(row[key] == jsonValue(value), "manifest", "%s row %v disagrees with the corpus", kind, row["id"])
	}
}

// joinRows: every corpus example, suite and control appears once, with matching
// data.
func joinRows(rows map[string]any, corpus *Corpus) {
	expectedIDs := map[string]bool{}
	for _, e := range corpus.Examples {
		expectedIDs[e.ID] = true
	}
	for _, s := range corpus.Suites {
		expectedIDs[s.ID] = true
	}
	for _, c := range corpus.Controls {
		expectedIDs[c.ID] = true
	}
	valid := len(rows) == len(expectedIDs)
	for id := range rows {
		valid = valid && expectedIDs[id]
	}
	need(valid, "manifest", "row ids differ from the corpus")
	for _, example := range corpus.Examples {
		row := manifestObject(rows[example.ID], rowKeys["example"], "example row")
		agrees(row, "example", map[string]any{"op": example.Op, "cases": 1})
	}
	for _, suite := range corpus.Suites {
		row := manifestObject(rows[suite.ID], rowKeys["exhaustive"], "exhaustive row")
		need(len(suite.Ops) == 1, "manifest", "profile allows one subject op per suite")
		agrees(row, "exhaustive", map[string]any{
			"op":              suite.Ops[0],
			"claim":           suite.Claim,
			"cases":           suite.Cases,
			"discarded":       suite.Discarded,
			"calls":           suite.ExpectedCalls,
			"domain":          suite.DomainText,
			"inputs_sha256":   suite.InputsSha256,
			"expected_sha256": suite.ExpectedSha256,
		})
	}
	for _, control := range corpus.Controls {
		row := manifestObject(rows[control.ID], rowKeys["negctl"], "negctl row")
		caught, isList := row["caught_by"].([]any)
		valid := isList && len(caught) > 0
		seen := map[string]bool{}
		for _, claim := range caught {
			text, isString := claim.(string)
			valid = valid && isString && !seen[text] && containsString(control.RequiredClaims, text)
			seen[text] = true
		}
		need(valid, "manifest", "caught_by must be a non-empty subset of the required claims")
		agrees(row, "negctl", map[string]any{"fault": control.Fault, "op": control.Op, "target": control.TargetKind})
	}
}

func joinRegistry(registryBytes []byte, lane, packageName, module string, requireActive bool) {
	registry, ok := parseJSON(registryBytes).(map[string]any)
	consumers, isList := []any(nil), false
	if ok {
		consumers, isList = registry["consumers"].([]any)
	}
	need(ok && isList, "registry", "bad registry")
	var matches []map[string]any
	for _, row := range consumers {
		if fields, isObj := row.(map[string]any); isObj && fields["lane"] == lane && fields["package"] == packageName {
			matches = append(matches, fields)
		}
	}
	need(len(matches) == 1, "registry", "lane/package is not registered exactly once")
	need(matches[0]["module"] == module, "registry", "module differs from the registered consumer")
	need(!requireActive || matches[0]["status"] == "active", "registry", "the registered consumer is not active")
}

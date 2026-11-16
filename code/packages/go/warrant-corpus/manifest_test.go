package warrantcorpus

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func writeTree(t *testing.T, root string, files map[string]string) {
	t.Helper()
	for name, content := range files {
		mustWrite(t, filepath.Join(root, filepath.FromSlash(name)), []byte(content))
	}
}

func TestReadSourceTreeSelectsProductionGoFiles(t *testing.T) {
	root := t.TempDir()
	writeTree(t, root, map[string]string{
		"a.go": "package a", "a_test.go": "package a", "sub/b.go": "package b", "sub/deep/c.go": "package c",
		"notes.md": "x", "vendor/v.go": "x", "testdata/t.go": "x", ".hidden/h.go": "x", "_ignored/i.go": "x",
		"cmd/x/main.go": "package main", "Z.go": "z",
	})
	var names []string
	for _, file := range readSourceTree(root) {
		names = append(names, file.Path)
	}
	if got := strings.Join(names, " "); got != "Z.go a.go cmd/x/main.go sub/b.go sub/deep/c.go" {
		t.Fatalf("files: %s", got)
	}
}

func TestSourceTreeRefusals(t *testing.T) {
	root := t.TempDir()
	writeTree(t, root, map[string]string{"a.go": "package a"})
	if err := os.Symlink(filepath.Join(root, "a.go"), filepath.Join(root, "link.go")); err == nil {
		wantCode(t, "source-tree", func() { readSourceTree(root) })
		os.Remove(filepath.Join(root, "link.go"))
	}
	wantCode(t, "source-tree", func() { sourceDigest(filepath.Join(root, "missing")) })
	wantCode(t, "source-tree", func() { sourceDigest(filepath.Join(root, "a.go")) })
	deep := root
	for i := 0; i < maxSourceDepth+2; i++ {
		deep = filepath.Join(deep, "d")
	}
	writeTree(t, deep, map[string]string{"x.go": "package x"})
	wantCode(t, "source-tree", func() { readSourceTree(root) })
	big := t.TempDir()
	if err := os.WriteFile(filepath.Join(big, "big.go"), make([]byte, maxSourceFileBytes+1), 0o644); err != nil {
		t.Fatal(err)
	}
	wantCode(t, "source-tree", func() { readSourceTree(big) })
}

func TestFrameSourcesIsLengthPrefixed(t *testing.T) {
	got := frameSources([]SourceFile{{"b", []byte("2")}, {"a", []byte("1")}})
	want := "warrant-src-v1\x00" + "\x02\x00\x00\x00\x00\x00\x00\x00" +
		"\x01\x00\x00\x00\x00\x00\x00\x00a" + "\x01\x00\x00\x00\x00\x00\x00\x001" +
		"\x01\x00\x00\x00\x00\x00\x00\x00b" + "\x01\x00\x00\x00\x00\x00\x00\x002"
	if string(got) != want {
		t.Fatalf("%q", got)
	}
	// Moving a byte between a path and its content changes the digest.
	one := sha256Hex(frameSources([]SourceFile{{"ab", []byte("c")}}))
	two := sha256Hex(frameSources([]SourceFile{{"a", []byte("bc")}}))
	if one == two {
		t.Fatal("framing must be unambiguous")
	}
}

func TestRunnerDigestCoversTheCompiledSource(t *testing.T) {
	// The embedded copy and the on-disk package agree, so the digest recorded in a
	// manifest describes the files in this directory.
	if got, want := runnerDigest(), sourceDigest("."); got != want {
		t.Fatalf("embedded %s != on-disk %s", got, want)
	}
}

func TestToolchainVersionHasNoGoPrefix(t *testing.T) {
	if v := toolchainVersion(); strings.HasPrefix(v, "go") || v == "" {
		t.Fatalf("%q", v)
	}
}

func fullManifest(t *testing.T, f *fixture) (*Corpus, map[string]any, string) {
	t.Helper()
	p := prepared(t, f, nil)
	outcome := execute(p, func(string) {})
	if outcome.Manifest == nil {
		t.Fatal("no manifest")
	}
	return p.Corpus, outcome.Manifest, p.CorpusRelative
}

func TestManifestShapeAndValidation(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	corpus, manifest, relative := fullManifest(t, f)
	if relative != "code/specs/fixtures/ct-compare-warrant-v1" || manifest["lane"] != "go" || manifest["faulted"] != false ||
		manifest["evidence_seed"] != "6287397068022371361" || manifest["schema"] != ManifestSchema {
		t.Fatalf("%v", manifest)
	}
	ids := []string{}
	for _, row := range manifest["tests"].([]map[string]any) {
		ids = append(ids, row["id"].(string))
	}
	for i := 1; i < len(ids); i++ {
		if ids[i-1] >= ids[i] {
			t.Fatal("rows must be sorted by id")
		}
	}
	text := []byte(renderJSON(manifest))
	validated := validateManifest(text, corpus, relative, nil, false)
	if validated["lane"] != "go" {
		t.Fatal("validate")
	}
	// The checked-in registry lists go/ct-compare as active since WD-004b1, so a
	// Go manifest joins it, with and without --require-active.
	registry := mustRead(t, filepath.Join(f.corpusDir, "consumers.json"))
	validateManifest(text, corpus, relative, registry, false)
	validateManifest(text, corpus, relative, registry, true)
	// The unregistered and planned cases are built from the registry, not
	// assumed of it: drop the Go row, then put it back as planned.
	dropped := strings.Replace(string(registry), "go/ct-compare", "go/other-compare", 1)
	wantCode(t, "registry", func() { validateManifest(text, corpus, relative, []byte(dropped), false) })
	planned := strings.Replace(string(registry), `"status": "active"`, `"status": "planned"`, 1)
	validateManifest(text, corpus, relative, []byte(planned), false)
	wantCode(t, "registry", func() { validateManifest(text, corpus, relative, []byte(planned), true) })
}

// mutateManifest re-renders the manifest with change applied to a parsed copy.
func mutateManifest(t *testing.T, manifest map[string]any, change func(m map[string]any)) []byte {
	t.Helper()
	copyOf := parseJSON([]byte(renderJSON(manifest))).(map[string]any)
	change(copyOf)
	return []byte(renderJSON(copyOf))
}

func row(m map[string]any, i int) map[string]any { return m["tests"].([]any)[i].(map[string]any) }

func rowByKind(m map[string]any, kind string) map[string]any {
	for _, r := range m["tests"].([]any) {
		if r.(map[string]any)["kind"] == kind {
			return r.(map[string]any)
		}
	}
	return nil
}

func TestValidateManifestRefusals(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	corpus, manifest, relative := fullManifest(t, f)
	cases := map[string]func(m map[string]any){
		"extra key":         func(m map[string]any) { m["extra"] = "x" },
		"schema":            func(m map[string]any) { m["schema"] = "other" },
		"lane":              func(m map[string]any) { m["lane"] = "Go!" },
		"package lane":      func(m map[string]any) { m["package"] = "python/ct-compare" },
		"package shape":     func(m map[string]any) { m["package"] = "go/a/b" },
		"package name":      func(m map[string]any) { m["package"] = "go/Bad Name" },
		"module":            func(m map[string]any) { m["module"] = "other" },
		"spec":              func(m map[string]any) { m["spec"] = "code/specs/other.warrant" },
		"spec digest":       func(m map[string]any) { m["spec_sha256"] = strings.Repeat("0", 64) },
		"corpus path":       func(m map[string]any) { m["corpus"] = "elsewhere" },
		"corpus digest":     func(m map[string]any) { m["corpus_sha256"] = strings.Repeat("0", 64) },
		"cases digest":      func(m map[string]any) { m["cases_sha256"] = strings.Repeat("0", 64) },
		"runner keys":       func(m map[string]any) { m["runner"].(map[string]any)["x"] = "y" },
		"runner name":       func(m map[string]any) { m["runner"].(map[string]any)["name"] = "other" },
		"runner lane":       func(m map[string]any) { m["runner"].(map[string]any)["lane"] = "python" },
		"runner digest":     func(m map[string]any) { m["runner"].(map[string]any)["src_sha256"] = "zz" },
		"binding digest":    func(m map[string]any) { m["binding_sha256"] = "zz" },
		"source digest":     func(m map[string]any) { m["source_sha256"] = int64(1) },
		"toolchain keys":    func(m map[string]any) { m["toolchain"].(map[string]any)["x"] = "y" },
		"toolchain types":   func(m map[string]any) { m["toolchain"].(map[string]any)["version"] = int64(3) },
		"seed type":         func(m map[string]any) { m["evidence_seed"] = int64(5) },
		"seed leading zero": func(m map[string]any) { m["evidence_seed"] = "05" },
		"seed overflow":     func(m map[string]any) { m["evidence_seed"] = "18446744073709551616" },
		"seed not decimal":  func(m map[string]any) { m["evidence_seed"] = "0x5" },
		"faulted":           func(m map[string]any) { m["faulted"] = true },
		"tests not a list":  func(m map[string]any) { m["tests"] = "x" },
		"row dropped":       func(m map[string]any) { m["tests"] = m["tests"].([]any)[1:] },
		"row without id":    func(m map[string]any) { delete(row(m, 0), "id") },
		"row id not string": func(m map[string]any) { row(m, 0)["id"] = int64(1) },
		"row not an object": func(m map[string]any) { m["tests"].([]any)[0] = "x" },
		"duplicate id":      func(m map[string]any) { row(m, 1)["id"] = row(m, 0)["id"] },
		"unsorted":          func(m map[string]any) { t := m["tests"].([]any); t[0], t[1] = t[1], t[0] },
		"unknown id":        func(m map[string]any) { row(m, 0)["id"] = "warrant/ct_compare/example/aaa" },
		"example keys":      func(m map[string]any) { rowByKind(m, "example")["extra"] = "x" },
		"example op":        func(m map[string]any) { rowByKind(m, "example")["op"] = "eq" },
		"example cases":     func(m map[string]any) { rowByKind(m, "example")["cases"] = int64(2) },
		"example verdict":   func(m map[string]any) { rowByKind(m, "example")["verdict"] = "fail" },
		"example layer":     func(m map[string]any) { rowByKind(m, "example")["layer"] = "native" },
		"suite cases":       func(m map[string]any) { rowByKind(m, "exhaustive")["cases"] = int64(1) },
		"suite digest":      func(m map[string]any) { rowByKind(m, "exhaustive")["inputs_sha256"] = strings.Repeat("0", 64) },
		"suite claim":       func(m map[string]any) { rowByKind(m, "exhaustive")["claim"] = "x" },
		"suite domain":      func(m map[string]any) { rowByKind(m, "exhaustive")["domain"] = "x" },
		"control caught_by": func(m map[string]any) { rowByKind(m, "negctl")["caught_by"] = []any{} },
		"control foreign": func(m map[string]any) {
			rowByKind(m, "negctl")["caught_by"] = []any{"warrant/ct_compare/exhaustive/eq_agrees"}
		},
		"control duplicate": func(m map[string]any) {
			c := rowByKind(m, "negctl")["caught_by"].([]any)
			rowByKind(m, "negctl")["caught_by"] = append(c, c[0])
		},
		"control not list":   func(m map[string]any) { rowByKind(m, "negctl")["caught_by"] = "x" },
		"control non-string": func(m map[string]any) { rowByKind(m, "negctl")["caught_by"] = []any{int64(1)} },
		"control short name": func(m map[string]any) { rowByKind(m, "negctl")["caught_by"] = []any{"eq_fixed_agrees_w2"} },
		"control fault":      func(m map[string]any) { rowByKind(m, "negctl")["fault"] = "invert" },
		"control target":     func(m map[string]any) { rowByKind(m, "negctl")["target"] = "property" },
	}
	for name, change := range cases {
		text := mutateManifest(t, manifest, change)
		if code := laneCode(t, func() { validateManifest(text, corpus, relative, nil, false) }); code != "manifest" && code != "malformed-json" {
			t.Errorf("%s: code %q", name, code)
		}
	}
	wantCode(t, "malformed-json", func() { validateManifest([]byte("not json"), corpus, relative, nil, false) })
}

func TestValidateManifestRegistryJoin(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	corpus, manifest, relative := fullManifest(t, f)
	text := []byte(renderJSON(manifest))
	registry := func(status string, module string, count int) []byte {
		var rows []string
		for i := 0; i < count; i++ {
			rows = append(rows, `{"lane":"go","package":"go/ct-compare","module":"`+module+`","status":"`+status+`"}`)
		}
		return []byte(`{"consumers":[` + strings.Join(rows, ",") + `]}`)
	}
	wantCode(t, "", func() { validateManifest(text, corpus, relative, registry("planned", "ct_compare", 1), false) })
	wantCode(t, "", func() { validateManifest(text, corpus, relative, registry("active", "ct_compare", 1), true) })
	wantCode(t, "registry", func() { validateManifest(text, corpus, relative, registry("planned", "ct_compare", 1), true) })
	wantCode(t, "registry", func() { validateManifest(text, corpus, relative, registry("active", "other", 1), false) })
	wantCode(t, "registry", func() { validateManifest(text, corpus, relative, registry("active", "ct_compare", 0), false) })
	wantCode(t, "registry", func() { validateManifest(text, corpus, relative, registry("active", "ct_compare", 2), false) })
	wantCode(t, "registry", func() { validateManifest(text, corpus, relative, []byte(`[]`), false) })
	wantCode(t, "registry", func() { validateManifest(text, corpus, relative, []byte(`{"consumers":5}`), false) })
	wantCode(t, "registry", func() { validateManifest(text, corpus, relative, []byte(`{"consumers":[5]}`), false) })
}

func TestLabels(t *testing.T) {
	for descriptor, want := range map[string]string{
		"bool": "full", "u8_all": "full", "bytes_n(0)": "full", "bytes_n(1)": "full", "bytes_n(01)": "full",
		"bytes_n(2)": "bounded-set", "bytes_n(16)": "bounded-set", "bytes_n(99999999999999999999)": "bounded-set",
		"u64_edge": "bounded-set", "bytes_d1": "bounded-set", "nat_0_64": "bounded-set",
	} {
		if got := domainLabel(descriptor); got != want {
			t.Errorf("%s: %s, want %s", descriptor, got, want)
		}
	}
	wantCode(t, "unsupported-domain", func() { domainLabel("nope") })
	corpus := realCorpusLoaded(t)
	labels := deriveLabels(corpus, "go/ct-compare", "ct_compare", strings.Repeat("a", 64))
	byLabel := labels["cases_by_label"].(map[string]any)
	if byLabel["full"] != 65537 || byLabel["bounded-set"] != 506572 {
		t.Fatalf("LP-012 headline: %v", byLabel)
	}
	suites := labels["suites"].([]map[string]any)
	if len(suites) != 9 || suites[0]["id"].(string) > suites[1]["id"].(string) {
		t.Fatal("suites are sorted by id")
	}
}

func TestLabelDigestIsBoundToTheManifestsToolchainName(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	corpus, manifest, _ := fullManifest(t, f)
	base := labelsFor(corpus, manifest)["manifest_sha256"]
	clone := func(name, version string) map[string]any {
		m := parseJSON([]byte(renderJSON(manifest))).(map[string]any)
		m["toolchain"] = map[string]any{"name": name, "version": version}
		return m
	}
	// Same name, different version: not drift. Different name: drift. And a
	// manifest from another lane's toolchain is bound to ITS OWN name, not to a
	// hard-coded one.
	if labelsFor(corpus, clone("go", "9.9.9"))["manifest_sha256"] != base {
		t.Fatal("toolchain.version must not move the digest")
	}
	node := labelsFor(corpus, clone("node", "22"))["manifest_sha256"]
	if node == base || node != labelsFor(corpus, clone("node", "23"))["manifest_sha256"] {
		t.Fatal("toolchain.name must be part of the digest, version not")
	}
	// A manifest without a usable toolchain still yields a digest (validation
	// refuses it elsewhere).
	labelsFor(corpus, map[string]any{"package": 5, "module": 6})
}

func TestRealLaneManifestsFromOtherLanesAreValidated(t *testing.T) {
	// Every lane's runner must accept every other lane's manifest and label
	// sidecar (the lesson of WD-102). The committed Python and TypeScript
	// artifacts are real runs of the same corpus.
	corpus := realCorpusLoaded(t)
	for _, lane := range []string{"python", "typescript"} {
		dir := filepath.Join(realRepoRoot, "code", "packages", lane, "ct-compare", "evidence")
		manifestPath := filepath.Join(dir, "test-manifest.json")
		data, err := os.ReadFile(manifestPath)
		if err != nil {
			t.Skip("the " + lane + " lane artifact is not present")
		}
		document := validateManifest(data, corpus, "code/specs/fixtures/ct-compare-warrant-v1", nil, false)
		expected := renderJSON(labelsFor(corpus, document))
		if got := string(mustRead(t, filepath.Join(dir, LabelsFile))); got != expected {
			t.Errorf("%s: the label sidecar differs from the labels this runner derives", lane)
		}
	}
}

func TestPycacheIsolatedIsRequiredForPythonAndRefusedElsewhere(t *testing.T) {
	// VER04i 8.1: `pycache_isolated` is the one lane-specific key. Required, and
	// exactly true, for the Python lane; forbidden for every other lane.
	corpus := realCorpusLoaded(t)
	const relative = "code/specs/fixtures/ct-compare-warrant-v1"
	pythonPath := filepath.Join(realRepoRoot, "code", "packages", "python", "ct-compare", "evidence", "test-manifest.json")
	pythonBytes, err := os.ReadFile(pythonPath)
	if err != nil {
		t.Skip("the python lane artifact is not present")
	}
	python := parseJSON(pythonBytes).(map[string]any)
	if python["pycache_isolated"] != true {
		t.Fatal("the committed python manifest must record pycache_isolated: true")
	}
	refused := map[string]func(m map[string]any){
		"python without the key": func(m map[string]any) { delete(m, "pycache_isolated") },
		"python with false":      func(m map[string]any) { m["pycache_isolated"] = false },
		"python with a non-bool": func(m map[string]any) { m["pycache_isolated"] = "true" },
		"go with the key": func(m map[string]any) {
			m["lane"] = "go"
			m["package"] = "go/ct-compare"
			m["runner"].(map[string]any)["lane"] = "go"
		},
		"typescript with the key": func(m map[string]any) {
			m["lane"] = "typescript"
			m["package"] = "typescript/ct-compare"
			m["runner"].(map[string]any)["lane"] = "typescript"
		},
	}
	for name, change := range refused {
		text := mutateManifest(t, python, change)
		if code := laneCode(t, func() { validateManifest(text, corpus, relative, nil, false) }); code != "manifest" {
			t.Errorf("%s: code %q", name, code)
		}
	}
	// A non-Python manifest WITHOUT the key passes: the freshly built Go manifest.
	f := newFixture(t, fixtureOptions{mini: true})
	miniCorpus, manifest, miniRelative := fullManifest(t, f)
	if _, has := manifest["pycache_isolated"]; has {
		t.Fatal("the Go lane must not record pycache_isolated")
	}
	validateManifest([]byte(renderJSON(manifest)), miniCorpus, miniRelative, nil, false)
}

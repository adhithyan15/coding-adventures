package main

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/cache"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/executor"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

const (
	gameInputFrameCacheConsumer = "typescript/game-input-frame"
	gameInputFrameCacheBridge   = "rust/typescript-warrant-ir"
	gameInputFrameCacheParser   = "rust/typescript-parser"
	gameInputFrameCacheWIR      = "rust/warrant-ir"
	gameInputFrameCacheSolver   = "rust/bit-blaster"
)

type gameInputCachePackage struct {
	name, language, manifest, build, sourcePath, source string
}

func writeGameInputCacheFixture(t *testing.T) (string, []discovery.Package, *directedgraph.Graph) {
	t.Helper()
	root := t.TempDir()
	copyTypeScriptGameInputFrameInputs(t, root)
	copyFixedPointReaderInputs(t, toolchainFixtureRepoRoot(t), root)
	fixtures := []gameInputCachePackage{
		{
			name: gameInputFrameCacheConsumer, language: "typescript",
			manifest:   `{"name":"@coding-adventures/game-input-frame","version":"0.1.0"}`,
			build:      "# needs-toolchain: rust\n# build-tool: deps=rust/typescript-warrant-ir\necho cache-probe",
			sourcePath: "src/digital.ts", source: "export const proof = 1;\n",
		},
		{
			name: gameInputFrameCacheBridge, language: "rust",
			manifest:   "[package]\nname = \"typescript-warrant-ir\"\nversion = \"0.1.0\"\n[dependencies]\ntypescript-parser = { path = \"../typescript-parser\" }\nwarrant-ir = { path = \"../warrant-ir\" }\n",
			build:      "# build-tool: deps=rust/bit-blaster rust/sha256\necho proof-tool",
			sourcePath: "src/lib.rs", source: "pub fn bridge() -> u8 { 1 }\n",
		},
		{
			name: gameInputFrameCacheParser, language: "rust",
			manifest: "[package]\nname = \"typescript-parser\"\nversion = \"0.1.0\"\n[dependencies]\ntypescript-lexer = { path = \"../typescript-lexer\" }\n",
			build:    "echo parser", sourcePath: "src/lib.rs", source: "pub fn parse() -> u8 { 1 }\n",
		},
		{
			name: gameInputFrameCacheWIR, language: "rust",
			manifest: "[package]\nname = \"warrant-ir\"\nversion = \"0.1.0\"\n",
			build:    "echo wir", sourcePath: "src/lib.rs", source: "pub fn lower() -> u8 { 1 }\n",
		},
		{
			name: gameInputFrameCacheSolver, language: "rust",
			manifest: "[package]\nname = \"bit-blaster\"\nversion = \"0.1.0\"\n",
			build:    "echo solver", sourcePath: "src/lib.rs", source: "pub fn solve() -> u8 { 1 }\n",
		},
		{
			name: "rust/typescript-lexer", language: "rust",
			manifest: "[package]\nname = \"typescript-lexer\"\nversion = \"0.1.0\"\n[dependencies]\nlexer = { path = \"../lexer\" }\n",
			build:    "echo lexer", sourcePath: "src/lib.rs", source: "pub fn lex() -> u8 { 1 }\n",
		},
		{
			name: "rust/lexer", language: "rust",
			manifest: "[package]\nname = \"lexer\"\nversion = \"0.1.0\"\n",
			build:    "echo lexer", sourcePath: "src/lib.rs", source: "pub fn scan() -> u8 { 1 }\n",
		},
		{
			name: "rust/sha256", language: "rust",
			manifest: "[package]\nname = \"sha256\"\nversion = \"0.1.0\"\n",
			build:    "echo sha256", sourcePath: "src/lib.rs", source: "pub fn digest() -> u8 { 1 }\n",
		},
		{
			name: "go/unrelated-cache-test", language: "go",
			manifest: "module example.com/unrelated\n\ngo 1.23\n",
			build:    "echo unrelated", sourcePath: "main.go", source: "package main\nfunc main() {}\n",
		},
	}

	packages := make([]discovery.Package, 0, len(fixtures))
	for _, fixture := range fixtures {
		parts := strings.SplitN(fixture.name, "/", 2)
		packagePath := filepath.Join(root, "code", "packages", parts[0], parts[1])
		if err := os.MkdirAll(filepath.Join(packagePath, filepath.Dir(fixture.sourcePath)), 0o755); err != nil {
			t.Fatal(err)
		}
		manifestName := "Cargo.toml"
		if fixture.language == "typescript" {
			manifestName = "package.json"
		}
		if err := os.WriteFile(filepath.Join(packagePath, manifestName), []byte(fixture.manifest), 0o644); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(packagePath, "BUILD"), []byte(fixture.build), 0o644); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(packagePath, filepath.FromSlash(fixture.sourcePath)), []byte(fixture.source), 0o644); err != nil {
			t.Fatal(err)
		}
		packages = append(packages, discovery.Package{
			Name: fixture.name, Path: packagePath, Language: fixture.language,
			BuildContent: fixture.build, BuildCommands: []string{fixture.build[strings.LastIndex(fixture.build, "\n")+1:]},
		})
	}
	ensureFixedPointReaderPackages(t, root)
	graph, _, err := resolver.ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatalf("resolve complete cache fixture: %v", err)
	}
	return root, packages, graph
}

func copyTypeScriptGameInputFrameInputs(t *testing.T, root string) {
	t.Helper()
	repositoryRoot := toolchainFixtureRepoRoot(t)
	for _, input := range typescriptGameInputFrameInputs {
		original, err := os.ReadFile(filepath.Join(repositoryRoot, filepath.FromSlash(input)))
		if err != nil {
			t.Fatalf("read checked external input %s: %v", input, err)
		}
		destination := filepath.Join(root, filepath.FromSlash(input))
		if err := os.MkdirAll(filepath.Dir(destination), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(destination, original, 0o644); err != nil {
			t.Fatal(err)
		}
	}
	// Keep CLI fixtures self-contained: the real workspace manifest contains
	// hundreds of crates, while this graph intentionally models only the proof
	// bridge and its declared transitive providers.
	workspaceManifest := `[workspace]
resolver = "2"
members = [
  "bit-blaster",
  "lexer",
  "sha256",
  "typescript-lexer",
  "typescript-parser",
  "typescript-warrant-ir",
  "warrant-ir",
]
`
	if err := os.WriteFile(filepath.Join(root, "code/packages/rust/Cargo.toml"), []byte(workspaceManifest), 0o644); err != nil {
		t.Fatal(err)
	}
	exemptions := "EXCLUDED code/packages/rust  # synthetic virtual Cargo workspace root has no package target\n"
	if err := os.WriteFile(filepath.Join(root, "code/BUILD-EXEMPTIONS"), []byte(exemptions), 0o644); err != nil {
		t.Fatal(err)
	}
}

func mutateTypeScriptGameInputFrameInput(t *testing.T, path string, original []byte) []byte {
	t.Helper()
	var before, after string
	replacementCount := 1
	switch path {
	case "code/specs/fixtures/game-input-frame-v1/cases.json":
		before, after = `"tick": "1"`, `"tick": "2"`
		replacementCount = 2 // Keep the first vector's input and expected frame aligned.
	case "code/packages/typescript/tsconfig.base.json":
		before, after = `"target": "ES2022"`, `"target": "ES2023"`
	case "code/packages/rust/Cargo.toml":
		before, after = `resolver = "2"`, `resolver = "1"`
	case "code/packages/rust/Cargo.lock":
		before, after = "version = 4\n", "version = 3\n"
	case "code/packages/rust/.cargo/config.toml", ".cargo/config.toml":
		before, after = `linker = "rust-lld"`, `linker = "lld-link"`
	default:
		t.Fatalf("no semantic mutation defined for external input %s", path)
	}
	mutated := strings.Replace(string(original), before, after, replacementCount)
	if mutated == string(original) {
		t.Fatalf("external mutation source text %q not found in %s", before, path)
	}
	if strings.HasSuffix(path, ".json") && !json.Valid([]byte(mutated)) {
		t.Fatalf("external input mutation is not valid JSON: %s", path)
	}
	if path == "code/specs/fixtures/game-input-frame-v1/cases.json" {
		var corpus struct {
			Cases []struct {
				Input struct {
					Tick string `json:"tick"`
				} `json:"input"`
				Expected struct {
					Frame struct {
						Tick string `json:"tick"`
					} `json:"frame"`
				} `json:"expected"`
			} `json:"cases"`
		}
		if err := json.Unmarshal([]byte(mutated), &corpus); err != nil {
			t.Fatalf("decode mutated game-input-frame corpus: %v", err)
		}
		if len(corpus.Cases) == 0 || corpus.Cases[0].Input.Tick != "2" ||
			corpus.Cases[0].Expected.Frame.Tick != corpus.Cases[0].Input.Tick {
			t.Fatalf("first vector is not semantically paired after mutation: input tick=%q expected tick=%q",
				corpus.Cases[0].Input.Tick, corpus.Cases[0].Expected.Frame.Tick)
		}
	}
	return []byte(mutated)
}

func mustReadFile(t *testing.T, path string) []byte {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func runGameInputFrameCacheProbe(
	t *testing.T,
	root string,
	allPackages []discovery.Package,
	graph *directedgraph.Graph,
) (string, map[string]string, map[string]string) {
	t.Helper()
	var consumer discovery.Package
	var executionPackages []discovery.Package
	for _, pkg := range allPackages {
		if pkg.Language == "typescript" {
			executionPackages = append(executionPackages, pkg)
		}
		if pkg.Name == gameInputFrameCacheConsumer {
			consumer = pkg
			break
		}
	}
	if consumer.Name == "" {
		t.Fatal("fixture is missing the TypeScript consumer")
	}
	if len(executionPackages) != 1 || executionPackages[0].Name != gameInputFrameCacheConsumer {
		t.Fatalf("TypeScript-only execution set = %#v, want only %s", executionPackages, gameInputFrameCacheConsumer)
	}
	hashInputs, err := cacheHashPackages(executionPackages, allPackages, graph)
	if err != nil {
		t.Fatalf("resolve selected cache hash closure: %v", err)
	}
	hashPackage := func(pkg discovery.Package) (string, error) {
		if pkg.Name == gameInputFrameCacheConsumer {
			return hasher.HashPackageWithRepositoryInputs(pkg, root, typescriptGameInputFrameInputs)
		}
		return hasher.HashPackage(pkg)
	}
	packageHashes, depsHashes, failed, err := hashPackagesAndDependencies(hashInputs, graph, hashPackage)
	if err != nil {
		t.Fatalf("hash cache inputs for %s: %v", failed, err)
	}
	if packageHashes["go/unrelated-cache-test"] != "" {
		t.Fatal("TypeScript-only fallback hashed an unrelated Go package outside the provider closure")
	}
	for _, provider := range []string{
		gameInputFrameCacheBridge, gameInputFrameCacheParser, gameInputFrameCacheWIR,
		gameInputFrameCacheSolver, "rust/sha256",
	} {
		if packageHashes[provider] == "" {
			t.Fatalf("TypeScript-only fallback cache omitted provider hash %s", provider)
		}
	}
	cachePath := filepath.Join(root, ".build-cache.json")
	buildCache := cache.New()
	buildCache.Load(cachePath)
	results := executor.ExecuteBuilds(
		executionPackages, graph, buildCache, packageHashes, depsHashes,
		false, false, 1, nil, nil, false,
	)
	result, ok := results[consumer.Name]
	if !ok {
		t.Fatalf("executor omitted consumer result: %#v", results)
	}
	if err := buildCache.Save(cachePath); err != nil {
		t.Fatal(err)
	}
	t.Logf("fallback cache probe: affectedSet=nil force=false execution-filter=typescript command=%q package_hash=%s deps_hash=%s status=%s",
		consumer.BuildCommands, packageHashes[consumer.Name], depsHashes[consumer.Name], result.Status)
	return result.Status, packageHashes, depsHashes
}

func TestPersistedCacheUsesCompleteTypeScriptProofToolClosure(t *testing.T) {
	root, allPackages, graph := writeGameInputCacheFixture(t)
	packageOrder := make([]string, len(allPackages))
	for index, pkg := range allPackages {
		packageOrder[index] = pkg.Name
	}
	t.Logf("complete cache package iteration order: %s", strings.Join(packageOrder, ","))
	if !graph.HasEdge(gameInputFrameCacheBridge, gameInputFrameCacheConsumer) ||
		!graph.HasEdge(gameInputFrameCacheParser, gameInputFrameCacheBridge) ||
		!graph.HasEdge(gameInputFrameCacheWIR, gameInputFrameCacheBridge) ||
		!graph.HasEdge(gameInputFrameCacheSolver, gameInputFrameCacheBridge) {
		t.Fatal("resolver omitted a direct or transitive proof-tool edge")
	}

	status, initialPackageHashes, initialDepsHashes := runGameInputFrameCacheProbe(t, root, allPackages, graph)
	if status != "built" {
		t.Fatalf("first cache probe status = %q, want built", status)
	}
	status, _, _ = runGameInputFrameCacheProbe(t, root, allPackages, graph)
	if status != "skipped" {
		t.Fatalf("unchanged persisted cache status = %q, want skipped", status)
	}
	bridgeControl, err := os.ReadFile(filepath.Join(root, "code/packages/rust/typescript-warrant-ir/src/lib.rs"))
	if err != nil {
		t.Fatal(err)
	}
	for _, input := range typescriptGameInputFrameInputs {
		inputPath := filepath.Join(root, filepath.FromSlash(input))
		originalInput, err := os.ReadFile(inputPath)
		if err != nil {
			t.Fatalf("read external input %s: %v", input, err)
		}
		mutatedInput := mutateTypeScriptGameInputFrameInput(t, input, originalInput)
		if len(mutatedInput) != len(originalInput) || bytes.Equal(mutatedInput, originalInput) {
			t.Fatalf("external input mutation %s must differ with equal byte length", input)
		}
		if err := os.WriteFile(inputPath, mutatedInput, 0o644); err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(bridgeControl, mustReadFile(t, filepath.Join(root, "code/packages/rust/typescript-warrant-ir/src/lib.rs"))) {
			t.Fatalf("direct proof provider changed during external input mutation %s", input)
		}
		t.Logf("external input mutation: path=%s bytes_before=%d bytes_after=%d sha256_before=%s sha256_after=%s provider_control_sha256=%s",
			input, len(originalInput), len(mutatedInput), digestHex(originalInput), digestHex(mutatedInput), digestHex(bridgeControl))
		status, changedHashes, _ := runGameInputFrameCacheProbe(t, root, allPackages, graph)
		if status != "built" || changedHashes[gameInputFrameCacheConsumer] == initialPackageHashes[gameInputFrameCacheConsumer] {
			t.Fatalf("external input mutation %s was not hashed: status=%q", input, status)
		}
		if err := os.WriteFile(inputPath, originalInput, 0o644); err != nil {
			t.Fatal(err)
		}
		status, restoredHashes, _ := runGameInputFrameCacheProbe(t, root, allPackages, graph)
		if status != "built" || restoredHashes[gameInputFrameCacheConsumer] != initialPackageHashes[gameInputFrameCacheConsumer] {
			t.Fatalf("restored input %s did not restore the original key: status=%q", input, status)
		}
		status, _, _ = runGameInputFrameCacheProbe(t, root, allPackages, graph)
		if status != "skipped" {
			t.Fatalf("restored input %s warm cache status = %q, want skipped", input, status)
		}
	}

	bridgeSource := filepath.Join(root, "code/packages/rust/typescript-warrant-ir/src/lib.rs")
	originalBridge, err := os.ReadFile(bridgeSource)
	if err != nil {
		t.Fatal(err)
	}
	mutatedBridge := strings.Replace(string(originalBridge), "{ 1 }", "{ 2 }", 1)
	if mutatedBridge == string(originalBridge) {
		t.Fatal("direct proof-tool mutation fixture did not match")
	}
	if err := os.WriteFile(bridgeSource, []byte(mutatedBridge), 0o644); err != nil {
		t.Fatal(err)
	}
	t.Logf("source mutation: path=rust/typescript-warrant-ir/src/lib.rs bytes_before=%d bytes_after=%d sha256_before=%s sha256_after=%s",
		len(originalBridge), len(mutatedBridge), digestHex(originalBridge), digestHex([]byte(mutatedBridge)))
	status, changedPackageHashes, changedDepsHashes := runGameInputFrameCacheProbe(t, root, allPackages, graph)
	if status != "built" || changedPackageHashes[gameInputFrameCacheConsumer] != initialPackageHashes[gameInputFrameCacheConsumer] ||
		changedDepsHashes[gameInputFrameCacheConsumer] == initialDepsHashes[gameInputFrameCacheConsumer] {
		t.Fatalf("same-path same-length direct tool mutation: status=%q package=%q deps=%q", status,
			changedPackageHashes[gameInputFrameCacheConsumer], changedDepsHashes[gameInputFrameCacheConsumer])
	}
	if err := os.WriteFile(bridgeSource, originalBridge, 0o644); err != nil {
		t.Fatal(err)
	}
	status, restoredPackageHashes, restoredDepsHashes := runGameInputFrameCacheProbe(t, root, allPackages, graph)
	if status != "built" || restoredPackageHashes[gameInputFrameCacheConsumer] != initialPackageHashes[gameInputFrameCacheConsumer] ||
		restoredDepsHashes[gameInputFrameCacheConsumer] != initialDepsHashes[gameInputFrameCacheConsumer] {
		t.Fatalf("restored direct tool digest did not rebuild the original successful state: status=%q", status)
	}
	status, _, _ = runGameInputFrameCacheProbe(t, root, allPackages, graph)
	if status != "skipped" {
		t.Fatalf("restored warm cache status = %q, want skipped", status)
	}

	for _, provider := range []struct{ name, relativePath string }{
		{gameInputFrameCacheParser, "code/packages/rust/typescript-parser/src/lib.rs"},
		{gameInputFrameCacheWIR, "code/packages/rust/warrant-ir/src/lib.rs"},
		{gameInputFrameCacheSolver, "code/packages/rust/bit-blaster/src/lib.rs"},
	} {
		providerSource := filepath.Join(root, filepath.FromSlash(provider.relativePath))
		originalProvider, err := os.ReadFile(providerSource)
		if err != nil {
			t.Fatal(err)
		}
		mutatedProvider := strings.Replace(string(originalProvider), "{ 1 }", "{ 2 }", 1)
		if mutatedProvider == string(originalProvider) {
			t.Fatalf("transitive %s mutation fixture did not match", provider.name)
		}
		t.Logf("source mutation: path=%s bytes_before=%d bytes_after=%d sha256_before=%s sha256_after=%s",
			provider.relativePath, len(originalProvider), len(mutatedProvider), digestHex(originalProvider), digestHex([]byte(mutatedProvider)))
		if err := os.WriteFile(providerSource, []byte(mutatedProvider), 0o644); err != nil {
			t.Fatal(err)
		}
		status, _, changedDeps := runGameInputFrameCacheProbe(t, root, allPackages, graph)
		if status != "built" || changedDeps[gameInputFrameCacheConsumer] == initialDepsHashes[gameInputFrameCacheConsumer] {
			t.Fatalf("transitive %s mutation was not cached: status=%q", provider.name, status)
		}
		if err := os.WriteFile(providerSource, originalProvider, 0o644); err != nil {
			t.Fatal(err)
		}
		status, _, restoredDeps := runGameInputFrameCacheProbe(t, root, allPackages, graph)
		if status != "built" || restoredDeps[gameInputFrameCacheConsumer] != initialDepsHashes[gameInputFrameCacheConsumer] {
			t.Fatalf("restored %s digest was not recognized: status=%q", provider.name, status)
		}
		status, _, _ = runGameInputFrameCacheProbe(t, root, allPackages, graph)
		if status != "skipped" {
			t.Fatalf("restored %s warm cache status = %q, want skipped", provider.name, status)
		}
	}

	tsSource := filepath.Join(root, "code/packages/typescript/game-input-frame/src/digital.ts")
	originalTS, err := os.ReadFile(tsSource)
	if err != nil {
		t.Fatal(err)
	}
	mutatedTS := strings.Replace(string(originalTS), "= 1", "= 2", 1)
	if mutatedTS == string(originalTS) {
		t.Fatal("TypeScript positive-control mutation fixture did not match")
	}
	if err := os.WriteFile(tsSource, []byte(mutatedTS), 0o644); err != nil {
		t.Fatal(err)
	}
	t.Logf("source mutation: path=typescript/game-input-frame/src/digital.ts bytes_before=%d bytes_after=%d sha256_before=%s sha256_after=%s",
		len(originalTS), len(mutatedTS), digestHex(originalTS), digestHex([]byte(mutatedTS)))
	status, tsChangedPackageHashes, _ := runGameInputFrameCacheProbe(t, root, allPackages, graph)
	if status != "built" || tsChangedPackageHashes[gameInputFrameCacheConsumer] == initialPackageHashes[gameInputFrameCacheConsumer] {
		t.Fatalf("package-local TypeScript source positive control failed: status=%q", status)
	}
	if err := os.WriteFile(tsSource, originalTS, 0o644); err != nil {
		t.Fatal(err)
	}
	status, _, _ = runGameInputFrameCacheProbe(t, root, allPackages, graph)
	if status != "built" {
		t.Fatalf("restored TypeScript control should return to original digest, got %q", status)
	}
	status, _, _ = runGameInputFrameCacheProbe(t, root, allPackages, graph)
	if status != "skipped" {
		t.Fatalf("TypeScript control warm cache status = %q, want skipped", status)
	}
}

func TestRepositoryProofToolDiffClosureSurvivesTypeScriptExecutionFilter(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	packages, err := discovery.DiscoverPackages(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	packages = packagesForPlatform(packages, runtime.GOOS)
	graph, triggers, err := resolver.ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatalf("resolve live repository package graph: %v", err)
	}
	closure := []string{
		gameInputFrameCacheConsumer,
		gameInputFrameCacheBridge,
		gameInputFrameCacheParser,
		gameInputFrameCacheWIR,
		gameInputFrameCacheSolver,
		"rust/sha256",
	}
	for _, name := range closure {
		if !graph.HasNode(name) {
			t.Fatalf("repository graph is missing required proof package %s", name)
		}
	}
	for _, edge := range [][2]string{
		{gameInputFrameCacheBridge, gameInputFrameCacheConsumer},
		{gameInputFrameCacheSolver, gameInputFrameCacheBridge},
		{"rust/sha256", gameInputFrameCacheBridge},
		{gameInputFrameCacheParser, gameInputFrameCacheBridge},
		{gameInputFrameCacheWIR, gameInputFrameCacheBridge},
		{"rust/typescript-lexer", gameInputFrameCacheParser},
		{"rust/lexer", "rust/typescript-lexer"},
	} {
		if !graph.HasEdge(edge[0], edge[1]) {
			t.Fatalf("repository graph omitted expected proof-tool edge %s -> %s", edge[0], edge[1])
		}
	}
	for _, input := range []string{
		"code/packages/rust/typescript-warrant-ir/examples/source-proof.rs",
		"code/packages/rust/typescript-parser/src/lib.rs",
		"code/packages/rust/warrant-ir/src/lib.rs",
		"code/packages/rust/bit-blaster/src/lib.rs",
	} {
		roots, _, err := selectPackageRoots([]string{input}, nil, packages, root, runtime.GOOS, "all")
		if err != nil {
			t.Fatalf("select package root for %s: %v", input, err)
		}
		if len(roots) != 1 {
			t.Fatalf("changed tool file %s roots = %#v, want exactly one provider", input, roots)
		}
		affected := affectedForGraphWithTriggers(graph, triggers, roots, nil, false)
		if !affected[gameInputFrameCacheConsumer] {
			t.Fatalf("changed tool file %s does not select %s", input, gameInputFrameCacheConsumer)
		}
		t.Logf("diff selection: changed=%s roots=%v affected_count=%d consumer_selected=%t language_filter=typescript",
			input, affectedListFromSet(roots), len(affected), affected[gameInputFrameCacheConsumer])
	}
	var selected []discovery.Package
	for _, pkg := range packages {
		if pkg.Language == "typescript" {
			selected = append(selected, pkg)
		}
	}
	if !containsPackageName(selected, gameInputFrameCacheConsumer) {
		t.Fatal("TypeScript-only execution filter removed the game-input-frame consumer")
	}
}

func containsPackageName(packages []discovery.Package, name string) bool {
	for _, pkg := range packages {
		if pkg.Name == name {
			return true
		}
	}
	return false
}

func digestHex(source []byte) string {
	digest := sha256.Sum256(source)
	return hex.EncodeToString(digest[:])
}

package executor

import (
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"testing"
	"time"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/cache"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// containsString reports whether want appears in keys.
func containsString(keys []string, want string) bool {
	for _, k := range keys {
		if k == want {
			return true
		}
	}
	return false
}

// makeFixture creates a temporary directory tree for testing.
func makeFixture(t *testing.T, tree map[string]string) string {
	t.Helper()
	root := t.TempDir()
	for relPath, content := range tree {
		absPath := filepath.Join(root, filepath.FromSlash(relPath))
		if err := os.MkdirAll(filepath.Dir(absPath), 0755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(absPath, []byte(content), 0644); err != nil {
			t.Fatal(err)
		}
	}
	return root
}

// ---------------------------------------------------------------------------
// Tests for shellCommandForOS
// ---------------------------------------------------------------------------

func TestShellCommandForOSUnix(t *testing.T) {
	// On Unix (darwin, linux), shellCommandForOS should use "sh -c".
	cmd := shellCommandForOS("echo hello", "darwin")
	if cmd.Path == "" {
		t.Fatal("expected non-empty path")
	}
	if cmd.Args[0] != "sh" || cmd.Args[1] != "-c" || cmd.Args[2] != "echo hello" {
		t.Fatalf("expected sh -c 'echo hello', got %v", cmd.Args)
	}

	cmd = shellCommandForOS("echo hello", "linux")
	if cmd.Args[0] != "sh" || cmd.Args[1] != "-c" {
		t.Fatalf("expected sh -c on linux, got %v", cmd.Args)
	}
}

func TestShellCommandForOSWindows(t *testing.T) {
	// On Windows, shellCommandForOS should use "cmd /C".
	cmd := shellCommandForOS("echo hello", "windows")
	if cmd.Args[0] != "cmd" || cmd.Args[1] != "/C" || cmd.Args[2] != "echo hello" {
		t.Fatalf("expected cmd /C 'echo hello', got %v", cmd.Args)
	}
}

// TestShellCommandPreservesEmbeddedQuotesOnWindows is a regression test for
// a real bug: Go's os/exec re-escapes embedded double quotes when building
// the native Windows command line from Args, as if each Args element were a
// single literal argument. That corrupts any BUILD command that itself
// relies on shell-style quoting (e.g. `uv pip install -e ".[dev]"`), which
// arrives at the target program as the literal text `-e \".[dev]\"` instead
// of `-e .[dev]` -- see executor_windows.go's setRawWindowsCmdLine. This
// only exercises real behavior when actually running on Windows (the fix is
// a no-op on other platforms, matching how the bug itself is Windows-only).
func TestShellCommandPreservesEmbeddedQuotesOnWindows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("this regression only manifests in a real Windows process launch")
	}

	cmd := shellCommand(`echo before ".[dev]" after`)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("command failed: %v (output: %q)", err, out)
	}

	if strings.Contains(string(out), `\"`) {
		t.Fatalf("embedded quotes were escaped with a backslash, output: %q", out)
	}
	if !strings.Contains(string(out), `".[dev]"`) {
		t.Fatalf("expected literal quoted .[dev] in output, got: %q", out)
	}
}

// TestShellCommandForOSWindowsRewritesInlineEnvPrefix is a regression test
// for a real CI failure: `RUSTDOCFLAGS="-D warnings" cargo doc -p widget
// --no-deps` (a POSIX inline environment-variable prefix, present in 17+
// Rust crates' BUILD files) is valid to `sh -c` but a syntax error to
// `cmd /C` -- cmd has no "set this variable for just the next command"
// notation, so it tried to read a variable literally named `RUSTDOCFLAGS`
// (there being no `=` immediately after `set`... no, wait: cmd actually
// choked trying to *run* `RUSTDOCFLAGS="-D` as a program name) and failed
// with `'RUSTDOCFLAGS' is not recognized as an internal or external
// command`. shellCommandForOS must translate the prefix into cmd's own
// `set "VAR=value"&& command` form before invoking cmd.exe.
func TestShellCommandForOSWindowsRewritesInlineEnvPrefix(t *testing.T) {
	cmd := shellCommandForOS(`RUSTDOCFLAGS="-D warnings" cargo doc -p widget --no-deps`, "windows")
	want := `set "RUSTDOCFLAGS=-D warnings"&& cargo doc -p widget --no-deps`
	if cmd.Args[2] != want {
		t.Fatalf("got %q, want %q", cmd.Args[2], want)
	}
}

// TestShellCommandForOSUnixLeavesInlineEnvPrefixAlone confirms the rewrite
// is Windows-only: on Unix the same RUSTDOCFLAGS line already works
// natively under `sh -c` and must pass through completely unchanged.
func TestShellCommandForOSUnixLeavesInlineEnvPrefixAlone(t *testing.T) {
	line := `RUSTDOCFLAGS="-D warnings" cargo doc -p widget --no-deps`
	cmd := shellCommandForOS(line, "linux")
	if cmd.Args[2] != line {
		t.Fatalf("got %q, want unchanged %q", cmd.Args[2], line)
	}
}

// TestRewriteInlineEnvPrefixForWindows is table-driven over both the
// shapes this repo's BUILD files actually use (quoted and bare values,
// including the real NPM_CONFIG_CACHE/PYTHONIOENCODING/PYTHONPATH
// prefixes alongside RUSTDOCFLAGS) and the shapes it must deliberately
// leave alone: command substitution, `&&`/`;`-chains, and multi-assignment
// lines. Every "leave alone" case here already carries a hand-written
// BUILD_windows override elsewhere in the repo (verified by hand before
// writing this fix) -- cmd.exe has no single-line translation for real
// shell control flow, so guessing one would be worse than the syntax
// error it already produces.
func TestRewriteInlineEnvPrefixForWindows(t *testing.T) {
	cases := []struct {
		name string
		in   string
		want string
	}{
		{
			name: "quoted value with a flag-shaped dash",
			in:   `RUSTDOCFLAGS="-D warnings" cargo doc -p widget --no-deps`,
			want: `set "RUSTDOCFLAGS=-D warnings"&& cargo doc -p widget --no-deps`,
		},
		{
			name: "bare value",
			in:   `PYTHONPATH=src python3 -m pytest tests`,
			want: `set "PYTHONPATH=src"&& python3 -m pytest tests`,
		},
		{
			name: "bare value with path separators",
			in:   `NPM_CONFIG_CACHE=.npm-cache npm install --silent`,
			want: `set "NPM_CONFIG_CACHE=.npm-cache"&& npm install --silent`,
		},
		{
			name: "quoted value with internal spaces",
			in:   `PYTHONIOENCODING="utf 8" uv run main.py`,
			want: `set "PYTHONIOENCODING=utf 8"&& uv run main.py`,
		},
		{
			name: "empty quoted value",
			in:   `FOO="" cargo test -p widget`,
			want: `set "FOO="&& cargo test -p widget`,
		},
		{
			name: "no prefix at all",
			in:   `cargo test -p widget`,
			want: `cargo test -p widget`,
		},
		{
			name: "command substitution -- left unchanged, needs BUILD_windows",
			in:   `REPO_ROOT=$(git rev-parse --show-toplevel) && echo "$REPO_ROOT"`,
			want: `REPO_ROOT=$(git rev-parse --show-toplevel) && echo "$REPO_ROOT"`,
		},
		{
			name: "expansion default inside the value -- left unchanged",
			in:   `PERL5LIB=lib:${PERL5LIB:-} prove -l t/`,
			want: `PERL5LIB=lib:${PERL5LIB:-} prove -l t/`,
		},
		{
			name: "two chained assignments -- left unchanged",
			in:   `DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1 DOTNET_CLI_HOME=$HOME dotnet test`,
			want: `DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1 DOTNET_CLI_HOME=$HOME dotnet test`,
		},
		{
			name: "subshell wrapper -- left unchanged",
			in:   `(cd ../widget && PYTHONPATH=src python3 -m pytest tests)`,
			want: `(cd ../widget && PYTHONPATH=src python3 -m pytest tests)`,
		},
		{
			name: "assignment with no following command",
			in:   `RUSTDOCFLAGS="-D warnings"`,
			want: `RUSTDOCFLAGS="-D warnings"`,
		},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			if got := rewriteInlineEnvPrefixForWindows(c.in); got != c.want {
				t.Errorf("rewriteInlineEnvPrefixForWindows(%q) = %q, want %q", c.in, got, c.want)
			}
		})
	}
}

func TestShellCommandUsesCurrentOS(t *testing.T) {
	// shellCommand (no OS parameter) should use the current platform.
	cmd := shellCommand("echo test")
	if runtime.GOOS == "windows" {
		if cmd.Args[0] != "cmd" {
			t.Fatalf("expected cmd on windows, got %v", cmd.Args[0])
		}
	} else {
		if cmd.Args[0] != "sh" {
			t.Fatalf("expected sh on unix, got %v", cmd.Args[0])
		}
	}
}

// ---------------------------------------------------------------------------
// Tests for runPackageBuild
// ---------------------------------------------------------------------------

func TestRunPackageBuildSuccess(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "echo hello",
	})

	pkg := discovery.Package{
		Name:          "python/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"echo hello"},
		Language:      "python",
	}

	result := runPackageBuild(pkg, false)
	if result.Status != "built" {
		t.Fatalf("expected built, got %s (stderr: %s)", result.Status, result.Stderr)
	}
	if result.ReturnCode != 0 {
		t.Fatalf("expected return code 0, got %d", result.ReturnCode)
	}
}

func TestRunPackageBuildFailure(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "exit 1",
	})

	pkg := discovery.Package{
		Name:          "python/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"exit 1"},
		Language:      "python",
	}

	result := runPackageBuild(pkg, false)
	if result.Status != "failed" {
		t.Fatalf("expected failed, got %s", result.Status)
	}
	if result.ReturnCode == 0 {
		t.Fatal("expected non-zero return code")
	}
}

func TestRunPackageBuildMultipleCommands(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "echo first\necho second",
	})

	pkg := discovery.Package{
		Name:          "python/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"echo first", "echo second"},
		Language:      "python",
	}

	result := runPackageBuild(pkg, false)
	if result.Status != "built" {
		t.Fatalf("expected built, got %s", result.Status)
	}
}

func TestRunPackageBuildStopsOnFailure(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "echo first\nexit 1\necho third",
	})

	pkg := discovery.Package{
		Name:          "python/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"echo first", "exit 1", "echo third"},
		Language:      "python",
	}

	result := runPackageBuild(pkg, false)
	if result.Status != "failed" {
		t.Fatalf("expected failed, got %s", result.Status)
	}
	// "third" should not appear in stdout because execution stops at the failure.
	if contains(result.Stdout, "third") {
		t.Fatal("should not have executed command after failure")
	}
}

func TestRunPackageBuildReturnsUnsupportedWithoutExecutingShell(t *testing.T) {
	root := t.TempDir()
	pkg := discovery.Package{
		Name: "elixir/native-demo",
		Path: root,
		BuildCommands: []string{
			"echo BUILD_TOOL_UNSUPPORTED:ELIXIR_WINDOWS_NIF_LINK_UNAVAILABLE -- skipped",
		},
		Language: "elixir",
	}

	result := runPackageBuild(pkg, false)
	if result.Status != "unsupported" {
		t.Fatalf("expected unsupported, got %s", result.Status)
	}
	if result.ReasonCode != "ELIXIR_WINDOWS_NIF_LINK_UNAVAILABLE" {
		t.Fatalf("unexpected reason code: %q", result.ReasonCode)
	}
	if result.Stdout != "" || result.Stderr != "" || result.ReturnCode != 0 {
		t.Fatalf("unsupported front executed or returned process output: %#v", result)
	}
}

func TestUnsupportedBuildCodeIsClosedAndRejectsMixedCommands(t *testing.T) {
	valid := []string{
		"echo BUILD_TOOL_UNSUPPORTED:ELIXIR_WINDOWS_METAL_UNAVAILABLE -- skipped",
	}
	if code, ok := unsupportedBuildCode(valid); !ok || code != "ELIXIR_WINDOWS_METAL_UNAVAILABLE" {
		t.Fatalf("valid protocol was not recognized: code=%q ok=%v", code, ok)
	}

	invalid := [][]string{
		{"echo BUILD_TOOL_UNSUPPORTED:lowercase -- skipped"},
		{"echo BUILD_TOOL_UNSUPPORTED:CODE"},
		{"echo BUILD_TOOL_UNSUPPORTED:CODE -- skipped && mix test"},
		{"echo BUILD_TOOL_UNSUPPORTED:CODE -- skipped", "mix test"},
		{"echo skipped for Windows"},
	}
	for _, commands := range invalid {
		if code, ok := unsupportedBuildCode(commands); ok {
			t.Fatalf("invalid protocol accepted: commands=%v code=%q", commands, code)
		}
	}
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(substr) == 0 ||
		(len(s) > 0 && len(substr) > 0 && containsHelper(s, substr)))
}

func containsHelper(s, sub string) bool {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}

// ---------------------------------------------------------------------------
// Tests for ExecuteBuilds
// ---------------------------------------------------------------------------

func TestExecuteBuildsSkipsCached(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "echo a",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"echo a"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")

	bc := cache.New()
	bc.Record("python/pkg-a", "hash-a", "deps-a", "success")

	results := ExecuteBuilds(packages, graph, bc, map[string]string{"python/pkg-a": "hash-a"}, map[string]string{"python/pkg-a": "deps-a"}, false, false, 1, nil, nil, false)

	if results["python/pkg-a"].Status != "skipped" {
		t.Fatalf("expected skipped, got %s", results["python/pkg-a"].Status)
	}
}

func TestExecuteBuildsForceOverridesCache(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "echo a",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"echo a"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")

	bc := cache.New()
	bc.Record("python/pkg-a", "hash-a", "deps-a", "success")

	results := ExecuteBuilds(packages, graph, bc, map[string]string{"python/pkg-a": "hash-a"}, map[string]string{"python/pkg-a": "deps-a"}, true, false, 1, nil, nil, false)

	if results["python/pkg-a"].Status != "built" {
		t.Fatalf("expected built (force), got %s", results["python/pkg-a"].Status)
	}
}

func TestExecuteBuildsDryRun(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "echo a",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"echo a"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")

	bc := cache.New()

	results := ExecuteBuilds(packages, graph, bc, map[string]string{"python/pkg-a": "hash-a"}, map[string]string{"python/pkg-a": "deps-a"}, false, true, 1, nil, nil, false)

	if results["python/pkg-a"].Status != "would-build" {
		t.Fatalf("expected would-build, got %s", results["python/pkg-a"].Status)
	}
}

func TestExecuteBuildsDepSkipped(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "exit 1",
		"pkg-b/BUILD": "echo b",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"exit 1"}, Language: "python"},
		{Name: "python/pkg-b", Path: filepath.Join(root, "pkg-b"), BuildCommands: []string{"echo b"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")
	graph.AddNode("python/pkg-b")
	graph.AddEdge("python/pkg-a", "python/pkg-b") // B depends on A

	bc := cache.New()

	results := ExecuteBuilds(packages, graph, bc,
		map[string]string{"python/pkg-a": "ha", "python/pkg-b": "hb"},
		map[string]string{"python/pkg-a": "da", "python/pkg-b": "db"},
		true, false, 1, nil, nil, false)

	if results["python/pkg-a"].Status != "failed" {
		t.Fatalf("expected pkg-a failed, got %s", results["python/pkg-a"].Status)
	}
	if results["python/pkg-b"].Status != "dep-skipped" {
		t.Fatalf("expected pkg-b dep-skipped, got %s", results["python/pkg-b"].Status)
	}
}

func TestExecuteBuildsPropagatesUnsupportedDependency(t *testing.T) {
	root := t.TempDir()
	packages := []discovery.Package{
		{
			Name: "elixir/native",
			Path: filepath.Join(root, "native"),
			BuildCommands: []string{
				"echo BUILD_TOOL_UNSUPPORTED:ELIXIR_WINDOWS_NIF_LINK_UNAVAILABLE -- skipped",
			},
			Language: "elixir",
		},
		{Name: "elixir/app", Path: filepath.Join(root, "app"), BuildCommands: []string{"mix test"}, Language: "elixir"},
	}
	for _, pkg := range packages {
		if err := os.MkdirAll(pkg.Path, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	graph := directedgraph.New()
	graph.AddNode("elixir/native")
	graph.AddNode("elixir/app")
	graph.AddEdge("elixir/native", "elixir/app")
	buildCache := cache.New()

	results := ExecuteBuilds(
		packages,
		graph,
		buildCache,
		map[string]string{"elixir/native": "a", "elixir/app": "b"},
		map[string]string{"elixir/native": "c", "elixir/app": "d"},
		true,
		false,
		1,
		nil,
		nil,
		false,
	)

	if results["elixir/native"].Status != "unsupported" {
		t.Fatalf("native status = %q", results["elixir/native"].Status)
	}
	app := results["elixir/app"]
	if app.Status != "dep-unsupported" || app.ReasonCode != "DEPENDENCY_UNSUPPORTED" {
		t.Fatalf("app result = %#v", app)
	}
	if !buildCache.NeedsBuild("elixir/native", "a", "c") || !buildCache.NeedsBuild("elixir/app", "b", "d") {
		t.Fatal("unsupported results must not enter the success cache")
	}
}

func TestExecuteBuildsParallelLevel(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "echo a",
		"pkg-b/BUILD": "echo b",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"echo a"}, Language: "python"},
		{Name: "python/pkg-b", Path: filepath.Join(root, "pkg-b"), BuildCommands: []string{"echo b"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")
	graph.AddNode("python/pkg-b")
	// No edges — both at level 0, can run in parallel.

	bc := cache.New()

	results := ExecuteBuilds(packages, graph, bc,
		map[string]string{"python/pkg-a": "ha", "python/pkg-b": "hb"},
		map[string]string{"python/pkg-a": "da", "python/pkg-b": "db"},
		true, false, 2, nil, nil, false)

	if results["python/pkg-a"].Status != "built" {
		t.Fatalf("expected pkg-a built, got %s", results["python/pkg-a"].Status)
	}
	if results["python/pkg-b"].Status != "built" {
		t.Fatalf("expected pkg-b built, got %s", results["python/pkg-b"].Status)
	}
}

func TestBuildResourceKeysIncludesSelfAndReferencedPackages(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"shared/BUILD": "echo shared",
		"pkg-a/BUILD":  "cd ../shared && npm install",
	})

	pkg := discovery.Package{
		Name:          "typescript/pkg-a",
		Path:          filepath.Join(root, "pkg-a"),
		BuildCommands: []string{"cd ../shared && npm install"},
		Language:      "typescript",
	}

	pathToPkg := map[string]string{
		filepath.Join(root, "pkg-a"):  "typescript/pkg-a",
		filepath.Join(root, "shared"): "typescript/shared",
	}

	keys := buildResourceKeys(pkg, pathToPkg)
	joined := strings.Join(keys, ",")
	if !strings.Contains(joined, "typescript/pkg-a") {
		t.Fatalf("expected keys to include self, got %v", keys)
	}
	if !strings.Contains(joined, "typescript/shared") {
		t.Fatalf("expected keys to include referenced package, got %v", keys)
	}
}

func TestBuildResourceKeysIncludesGlobalHexCacheForElixirDepsGet(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "mix deps.get && mix test",
	})

	pkg := discovery.Package{
		Name:          "elixir/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"mix deps.get && mix test"},
		Language:      "elixir",
	}

	keys := buildResourceKeys(pkg, map[string]string{
		filepath.Join(root, "pkg"): "elixir/pkg",
	})
	joined := strings.Join(keys, ",")
	if !strings.Contains(joined, "global:hex-cache") {
		t.Fatalf("expected keys to include global Hex cache lock, got %v", keys)
	}
}

func TestBuildResourceKeysIncludesGlobalRustupTargetLock(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "rustup target add wasm32-unknown-unknown && cargo build --target wasm32-unknown-unknown --release",
	})

	pkg := discovery.Package{
		Name:          "unknown/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"rustup target add wasm32-unknown-unknown && cargo build --target wasm32-unknown-unknown --release"},
		Language:      "unknown",
	}

	keys := buildResourceKeys(pkg, map[string]string{
		filepath.Join(root, "pkg"): "unknown/pkg",
	})
	joined := strings.Join(keys, ",")
	if !strings.Contains(joined, "global:rustup-targets") {
		t.Fatalf("expected keys to include global rustup target lock, got %v", keys)
	}
}

func TestBuildResourceKeysIncludesGlobalLuaRocksLockForLuaWritesOnWindows(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD_windows": "luarocks make --local coding-adventures-pkg-0.1.0-1.rockspec",
	})

	pkg := discovery.Package{
		Name:          "lua/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"luarocks make --local coding-adventures-pkg-0.1.0-1.rockspec"},
		Language:      "lua",
	}

	keys := buildResourceKeysForOS(pkg, map[string]string{
		filepath.Join(root, "pkg"): "lua/pkg",
	}, "windows")
	joined := strings.Join(keys, ",")
	if !strings.Contains(joined, "global:luarocks-tree") {
		t.Fatalf("expected keys to include global luarocks-tree lock, got %v", keys)
	}
}

func TestBuildResourceKeysIncludesGlobalLuaRocksLockForLuaWritesOnLinux(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "luarocks make --local coding-adventures-pkg-0.1.0-1.rockspec",
	})

	pkg := discovery.Package{
		Name:          "lua/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"luarocks make --local coding-adventures-pkg-0.1.0-1.rockspec"},
		Language:      "lua",
	}

	keys := buildResourceKeysForOS(pkg, map[string]string{
		filepath.Join(root, "pkg"): "lua/pkg",
	}, "linux")
	joined := strings.Join(keys, ",")
	if !strings.Contains(joined, "global:luarocks-tree") {
		t.Fatalf("expected keys to include global luarocks-tree lock on Linux, got %v", keys)
	}
}

func TestBuildResourceKeysIncludesGlobalLuaRocksLockForLuaRemovesOnLinux(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "luarocks remove --force coding-adventures-pkg 2>/dev/null || true",
	})

	pkg := discovery.Package{
		Name:          "lua/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"luarocks remove --force coding-adventures-pkg 2>/dev/null || true"},
		Language:      "lua",
	}

	keys := buildResourceKeysForOS(pkg, map[string]string{
		filepath.Join(root, "pkg"): "lua/pkg",
	}, "linux")
	joined := strings.Join(keys, ",")
	if !strings.Contains(joined, "global:luarocks-tree") {
		t.Fatalf("expected keys to include global luarocks-tree lock for remove commands, got %v", keys)
	}
}

func TestBuildResourceKeysIncludesGlobalCabalStoreLockForHaskellBuilds(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "if command -v cabal >/dev/null 2>&1; then cabal test; else echo 'cabal not found -- skipping'; fi",
	})

	pkg := discovery.Package{
		Name:          "haskell/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"if command -v cabal >/dev/null 2>&1; then cabal test; else echo 'cabal not found -- skipping'; fi"},
		Language:      "haskell",
	}

	keys := buildResourceKeys(pkg, map[string]string{
		filepath.Join(root, "pkg"): "haskell/pkg",
	})
	joined := strings.Join(keys, ",")
	if !strings.Contains(joined, "global:cabal-store") {
		t.Fatalf("expected keys to include global cabal-store lock, got %v", keys)
	}
}

func TestBuildResourceKeysDoesNotIncludeGlobalCabalStoreLockWithoutCabalCommand(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "echo haskell build skipped",
	})

	pkg := discovery.Package{
		Name:          "haskell/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"echo haskell build skipped"},
		Language:      "haskell",
	}

	keys := buildResourceKeys(pkg, map[string]string{
		filepath.Join(root, "pkg"): "haskell/pkg",
	})
	joined := strings.Join(keys, ",")
	if strings.Contains(joined, "global:cabal-store") {
		t.Fatalf("expected keys not to include global cabal-store lock, got %v", keys)
	}
}

func TestBuildResourceKeysIncludesGlobalDotnetLockForDotnetPackages(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "dotnet test tests/CodingAdventures.Tests.csproj --disable-build-servers",
	})

	pkg := discovery.Package{
		Name:          "csharp/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"dotnet test tests/CodingAdventures.Tests.csproj --disable-build-servers"},
		Language:      "csharp",
	}

	keys := buildResourceKeys(pkg, map[string]string{
		filepath.Join(root, "pkg"): "csharp/pkg",
	})
	joined := strings.Join(keys, ",")
	if !strings.Contains(joined, "global:dotnet-cli") {
		t.Fatalf("expected keys to include global dotnet-cli lock, got %v", keys)
	}
}

func TestBuildResourceKeysSkipsGlobalDotnetLockForNonDotnetPackages(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "echo dotnet test is only text here",
	})

	pkg := discovery.Package{
		Name:          "typescript/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"echo dotnet test is only text here"},
		Language:      "typescript",
	}

	keys := buildResourceKeys(pkg, map[string]string{
		filepath.Join(root, "pkg"): "typescript/pkg",
	})
	joined := strings.Join(keys, ",")
	if strings.Contains(joined, "global:dotnet-cli") {
		t.Fatalf("expected keys not to include global dotnet-cli lock, got %v", keys)
	}
}

func TestBuildResourceKeysIncludesGlobalGradleLockForJavaOnWindows(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "gradle test",
	})

	pkg := discovery.Package{
		Name:          "java/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"gradle test"},
		Language:      "java",
	}

	keys := buildResourceKeysForOS(pkg, map[string]string{
		filepath.Join(root, "pkg"): "java/pkg",
	}, "windows")
	joined := strings.Join(keys, ",")
	if !strings.Contains(joined, "global:gradle-windows-runner") {
		t.Fatalf("expected keys to include global gradle lock on Windows, got %v", keys)
	}
}

func TestBuildResourceKeysSkipsGlobalGradleLockForJavaOnLinux(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "gradle test",
	})

	pkg := discovery.Package{
		Name:          "java/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"gradle test"},
		Language:      "java",
	}

	keys := buildResourceKeysForOS(pkg, map[string]string{
		filepath.Join(root, "pkg"): "java/pkg",
	}, "linux")
	joined := strings.Join(keys, ",")
	if strings.Contains(joined, "global:gradle-windows-runner") {
		t.Fatalf("did not expect Windows-only gradle lock on Linux, got %v", keys)
	}
}

func TestBuildResourceKeysIncludesGlobalGradleLockForGradleWrapperOnWindows(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD_windows": "gradlew.bat assembleDebug",
	})

	pkg := discovery.Package{
		Name:          "kotlin/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"gradlew.bat assembleDebug"},
		Language:      "kotlin",
	}

	keys := buildResourceKeysForOS(pkg, map[string]string{
		filepath.Join(root, "pkg"): "kotlin/pkg",
	}, "windows")
	joined := strings.Join(keys, ",")
	if !strings.Contains(joined, "global:gradle-windows-runner") {
		t.Fatalf("expected wrapper-based builds to include global gradle lock on Windows, got %v", keys)
	}
}

func TestExecuteBuildsSerializesSharedBuildResources(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("uses shell commands that are only asserted on Unix runners")
	}

	root := makeFixture(t, map[string]string{
		"shared/BUILD": "echo shared",
		"pkg-a/BUILD":  "cd ../shared && mkdir .lockdir && sleep 1 && rmdir .lockdir",
		"pkg-b/BUILD":  "cd ../shared && mkdir .lockdir && sleep 1 && rmdir .lockdir",
	})

	packages := []discovery.Package{
		{Name: "typescript/shared", Path: filepath.Join(root, "shared"), BuildCommands: []string{"echo shared"}, Language: "typescript"},
		{Name: "typescript/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"cd ../shared && mkdir .lockdir && sleep 1 && rmdir .lockdir"}, Language: "typescript"},
		{Name: "typescript/pkg-b", Path: filepath.Join(root, "pkg-b"), BuildCommands: []string{"cd ../shared && mkdir .lockdir && sleep 1 && rmdir .lockdir"}, Language: "typescript"},
	}

	graph := directedgraph.New()
	for _, pkg := range packages {
		graph.AddNode(pkg.Name)
	}

	bc := cache.New()
	results := ExecuteBuilds(
		packages,
		graph,
		bc,
		map[string]string{
			"typescript/shared": "hs",
			"typescript/pkg-a":  "ha",
			"typescript/pkg-b":  "hb",
		},
		map[string]string{
			"typescript/shared": "ds",
			"typescript/pkg-a":  "da",
			"typescript/pkg-b":  "db",
		},
		true,
		false,
		3,
		nil,
		nil,
		false,
	)

	for _, name := range []string{"typescript/shared", "typescript/pkg-a", "typescript/pkg-b"} {
		if results[name].Status != "built" {
			t.Fatalf("expected %s to build successfully, got %s (stderr: %s)", name, results[name].Status, results[name].Stderr)
		}
	}
}

func TestExecuteBuildsCacheUpdatedOnSuccess(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "echo a",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"echo a"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")

	bc := cache.New()

	ExecuteBuilds(packages, graph, bc,
		map[string]string{"python/pkg-a": "hash-a"},
		map[string]string{"python/pkg-a": "deps-a"},
		true, false, 1, nil, nil, false)

	entries := bc.Entries()
	if entries["python/pkg-a"].Status != "success" {
		t.Fatalf("expected cache status success, got %s", entries["python/pkg-a"].Status)
	}
}

func TestExecuteBuildsCacheUpdatedOnFailure(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "exit 1",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"exit 1"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")

	bc := cache.New()

	ExecuteBuilds(packages, graph, bc,
		map[string]string{"python/pkg-a": "hash-a"},
		map[string]string{"python/pkg-a": "deps-a"},
		true, false, 1, nil, nil, false)

	entries := bc.Entries()
	if entries["python/pkg-a"].Status != "failed" {
		t.Fatalf("expected cache status failed, got %s", entries["python/pkg-a"].Status)
	}
}

// TestClippyGatedCommands verifies the clippy gate only injects a clippy
// command for Rust packages when enabled, and leaves everything else untouched.
func TestClippyGatedCommands(t *testing.T) {
	const clippyCmd = "cargo clippy --all-targets -- -D warnings"

	rustPkg := discovery.Package{
		Name:          "rust/bitset",
		Language:      "rust",
		BuildCommands: []string{"cargo test -p bitset"},
	}
	pyPkg := discovery.Package{
		Name:          "python/logic-gates",
		Language:      "python",
		BuildCommands: []string{"pytest"},
	}

	// Rust + clippy on: clippy command is prepended, BUILD command preserved.
	got := clippyGatedCommands(rustPkg, true)
	want := []string{clippyCmd, "cargo test -p bitset"}
	if len(got) != len(want) || got[0] != want[0] || got[1] != want[1] {
		t.Fatalf("rust+clippy: got %v, want %v", got, want)
	}

	// Rust + clippy off: unchanged.
	if got := clippyGatedCommands(rustPkg, false); len(got) != 1 || got[0] != "cargo test -p bitset" {
		t.Fatalf("rust+no-clippy: got %v, want [cargo test -p bitset]", got)
	}

	// Non-Rust + clippy on: never inject clippy.
	if got := clippyGatedCommands(pyPkg, true); len(got) != 1 || got[0] != "pytest" {
		t.Fatalf("python+clippy: got %v, want [pytest]", got)
	}

	// The gate must not mutate the package's own BuildCommands slice.
	if len(rustPkg.BuildCommands) != 1 || rustPkg.BuildCommands[0] != "cargo test -p bitset" {
		t.Fatalf("clippyGatedCommands mutated pkg.BuildCommands: %v", rustPkg.BuildCommands)
	}
}

// TestClippyStepFor covers how the clippy step mirrors a package's BUILD guard,
// so clippy only runs where the BUILD's own cargo invocation would run.
func TestClippyStepFor(t *testing.T) {
	const clippy = "cargo clippy --all-targets -- -D warnings"
	cases := []struct {
		name     string
		commands []string
		want     string
		wantOK   bool
	}{
		{
			name:     "unconditional cargo",
			commands: []string{"cargo test -p bitset -- --nocapture"},
			want:     clippy,
			wantOK:   true,
		},
		{
			name:     "unconditional cargo with tarpaulin follow-up",
			commands: []string{"cargo test -p vm-core", `if [ "$(uname)" = "Linux" ]; then cargo tarpaulin -p vm-core --out Stdout; fi`},
			want:     clippy,
			wantOK:   true,
		},
		{
			name:     "platform-guarded cargo (macOS)",
			commands: []string{`if [ "$(uname)" = "Darwin" ]; then cargo test -p paint-metal -- --nocapture; else echo "SKIP: paint-metal requires macOS/Apple platform"; fi`},
			want:     `if [ "$(uname)" = "Darwin" ]; then ` + clippy + `; fi`,
			wantOK:   true,
		},
		{
			name:     "pure echo skip (no cargo)",
			commands: []string{`echo "SKIP: paint-vm-direct2d requires Windows — not supported on Linux/macOS"`},
			want:     "",
			wantOK:   false,
		},
		{
			name:     "empty command list",
			commands: nil,
			want:     "",
			wantOK:   false,
		},
		// ── Regression: the cargo line is not the first line ──────────
		//
		// These are the shapes that silently disabled the gate when
		// clippyStepFor only inspected buildCommands[0]. Every one of them is
		// taken from a real BUILD file in this repo.
		{
			// code/packages/rust/sql-codegen/BUILD — `#!/bin/sh` is stripped as
			// a comment, so `set -e` is the first command the executor sees.
			name: "preamble before cargo (set -e + export)",
			commands: []string{
				"set -e",
				"export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld",
				"cargo test --package coding-adventures-sql-codegen",
			},
			want:   clippy,
			wantOK: true,
		},
		{
			// code/packages/rust/sql-vm/BUILD — progress `echo`s interleaved
			// with the cargo invocations.
			name: "echo progress lines before cargo",
			commands: []string{
				"set -e",
				`WORKSPACE="$(cd "$(dirname "$0")/.." && pwd)"`,
				`cd "$WORKSPACE"`,
				`echo "[sql-vm] Building package..."`,
				"cargo build --package coding-adventures-sql-vm",
			},
			want:   clippy,
			wantOK: true,
		},
		{
			// A guarded command that does NOT run cargo must not stop the scan:
			// the unconditional cargo further down still means "lint here".
			name: "non-cargo guard first, unconditional cargo later",
			commands: []string{
				`if [ -n "$CI" ]; then echo "CI build"; fi`,
				"cargo test -p widget",
			},
			want:   clippy,
			wantOK: true,
		},
		{
			// Unconditional beats guarded regardless of order: the crate
			// compiles on this platform, so the lint must not hide behind
			// someone else's platform guard.
			name: "guarded cargo first, unconditional cargo later",
			commands: []string{
				`if [ "$(uname)" = "Darwin" ]; then cargo build -p mac-extra; fi`,
				"cargo test -p widget",
			},
			want:   clippy,
			wantOK: true,
		},
		{
			// Still no cargo anywhere → still no clippy step. This is the case
			// that must NOT regress: it is what keeps the Linux/macOS legs from
			// trying to lint a Windows-only crate.
			name: "preamble but no cargo anywhere",
			commands: []string{
				"set -e",
				`echo "SKIP: paint-vm-gdi requires Windows — not supported on Linux/macOS"`,
			},
			want:   "",
			wantOK: false,
		},
		{
			// code/packages/rust/font-parser-node/BUILD — a compile-only crate
			// that PRINTS the cargo line it cannot run. Scanning every command
			// must not mistake documentation for an invocation.
			name: "cargo mentioned only inside an echo string",
			commands: []string{
				`echo "font-parser-node: compile-only crate (requires Node.js dev headers to link)"`,
				`echo "  To build: cargo build -p font-parser-node --release"`,
			},
			want:   "",
			wantOK: false,
		},
		{
			// Env-prefixed and `&&`-chained invocations are real cargo runs and
			// must still be recognised once quoted spans are erased.
			name:     "env-prefixed cargo doc",
			commands: []string{`RUSTDOCFLAGS="-D warnings" cargo doc -p widget --no-deps`},
			want:     clippy,
			wantOK:   true,
		},
		{
			name:     "cargo after cd and &&",
			commands: []string{`cd "$WORKSPACE" && cargo test --package widget`},
			want:     clippy,
			wantOK:   true,
		},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got, ok := clippyStepFor(tc.commands)
			if got != tc.want || ok != tc.wantOK {
				t.Fatalf("clippyStepFor(%v) = (%q, %v), want (%q, %v)", tc.commands, got, ok, tc.want, tc.wantOK)
			}
		})
	}
}

// --- B07: shared-directory safety -------------------------------------
//
// These cover the race that blocked PRs #15839 and #15858: a package that
// reads a sibling's node_modules through file: link resolution, scheduled
// beside a package that reinstalls into that same directory.

// writeTestPackageJSON writes a manifest with the given file: dependencies.
func writeTestPackageJSON(t *testing.T, dir string, fileDeps map[string]string) {
	t.Helper()
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatalf("mkdir %s: %v", dir, err)
	}
	deps := make(map[string]string, len(fileDeps))
	for name, rel := range fileDeps {
		deps[name] = "file:" + rel
	}
	body, err := json.Marshal(map[string]any{"dependencies": deps})
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	if err := os.WriteFile(filepath.Join(dir, "package.json"), body, 0o644); err != nil {
		t.Fatalf("write package.json: %v", err)
	}
}

// TestReadKeysCoverTransitiveFileDeps is the blog's exact shape: a BUILD
// naming no relative path, reaching three packages deep through file: links.
// Before B07 this package acquired no lock for any of them.
func TestReadKeysCoverTransitiveFileDeps(t *testing.T) {
	root := t.TempDir()
	blog := filepath.Join(root, "blog")
	cli := filepath.Join(root, "forme-cli")
	sm := filepath.Join(root, "state-machine")
	dg := filepath.Join(root, "directed-graph")

	writeTestPackageJSON(t, blog, map[string]string{"@ca/forme-cli": "../forme-cli"})
	writeTestPackageJSON(t, cli, map[string]string{"@ca/state-machine": "../state-machine"})
	writeTestPackageJSON(t, sm, map[string]string{"@ca/directed-graph": "../directed-graph"})
	writeTestPackageJSON(t, dg, nil)

	pkg := discovery.Package{
		Name:          "unknown/blog",
		Path:          blog,
		BuildCommands: []string{"npm install --silent && npm run clean && npm run build"},
	}
	pathToPkg := map[string]string{
		cli: "typescript/forme-cli",
		sm:  "typescript/state-machine",
		dg:  "typescript/directed-graph",
	}

	// The defect, stated directly: the text scan that produces WRITE keys
	// sees nothing here, because the BUILD names no relative path. This is
	// why the blog raced against packages reinstalling its dependencies.
	writeKeys := buildResourceKeys(pkg, pathToPkg)
	for _, unexpected := range []string{
		"typescript/directed-graph",
		"typescript/forme-cli",
		"typescript/state-machine",
	} {
		if containsString(writeKeys, unexpected) {
			t.Fatalf("precondition changed: write keys %v now contain %q, "+
				"so this test no longer covers the text-scan blind spot",
				writeKeys, unexpected)
		}
	}

	// What B07 adds: the same packages, reached through the file: closure.
	keys := buildReadResourceKeys(pkg, pathToPkg)
	for _, want := range []string{
		"typescript/directed-graph",
		"typescript/forme-cli",
		"typescript/state-machine",
	} {
		if !containsString(keys, want) {
			t.Errorf("read keys %v missing %q", keys, want)
		}
	}
}

// TestReadKeysSkipNonNPMPackages: a Rust or Go package cannot participate in
// the node_modules race, so its key set must be untouched by B07.
func TestReadKeysSkipNonNPMPackages(t *testing.T) {
	root := t.TempDir()
	crate := filepath.Join(root, "crate")
	dep := filepath.Join(root, "dep")
	writeTestPackageJSON(t, crate, map[string]string{"@ca/dep": "../dep"})
	writeTestPackageJSON(t, dep, nil)

	pkg := discovery.Package{
		Name:          "rust/crate",
		Path:          crate,
		BuildCommands: []string{"cargo test --all-features"},
	}
	if keys := buildReadResourceKeys(pkg, map[string]string{dep: "rust/dep"}); len(keys) != 0 {
		t.Errorf("non-npm package should take no read keys, got %v", keys)
	}
}

// TestReadKeysTolerateMissingManifest: key derivation runs for every package
// on every build, so an unreadable manifest must yield nothing, not a panic.
func TestReadKeysTolerateMissingManifest(t *testing.T) {
	root := t.TempDir()
	pkg := discovery.Package{
		Name:          "typescript/nope",
		Path:          filepath.Join(root, "absent"),
		BuildCommands: []string{"npm ci --quiet"},
	}
	if keys := buildReadResourceKeys(pkg, map[string]string{}); len(keys) != 0 {
		t.Errorf("missing manifest should yield no read keys, got %v", keys)
	}

	bad := filepath.Join(root, "bad")
	if err := os.MkdirAll(bad, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(bad, "package.json"), []byte("{not json"), 0o644); err != nil {
		t.Fatal(err)
	}
	pkg.Path = bad
	if keys := buildReadResourceKeys(pkg, map[string]string{}); len(keys) != 0 {
		t.Errorf("malformed manifest should yield no read keys, got %v", keys)
	}
}

// TestWriterExcludesReader is the property the whole change exists for.
func TestWriterExcludesReader(t *testing.T) {
	locker := newBuildResourceLocker()

	releaseWriter := locker.Acquire([]string{"typescript/state-machine"}, nil)

	readerIn := make(chan struct{})
	go func() {
		release := locker.Acquire(nil, []string{"typescript/state-machine"})
		close(readerIn)
		release()
	}()

	select {
	case <-readerIn:
		t.Fatal("reader entered while a writer held the same key")
	case <-time.After(50 * time.Millisecond):
	}

	releaseWriter()

	select {
	case <-readerIn:
	case <-time.After(2 * time.Second):
		t.Fatal("reader never entered after the writer released")
	}
}

// TestReadersShareAKey: the reason this uses RWMutex at all. 232 packages
// read typescript/directed-graph; exclusive reader locks would serialise
// nearly the whole TypeScript tree.
func TestReadersShareAKey(t *testing.T) {
	locker := newBuildResourceLocker()

	releaseFirst := locker.Acquire(nil, []string{"typescript/directed-graph"})
	defer releaseFirst()

	secondIn := make(chan struct{})
	go func() {
		release := locker.Acquire(nil, []string{"typescript/directed-graph"})
		close(secondIn)
		release()
	}()

	select {
	case <-secondIn:
	case <-time.After(2 * time.Second):
		t.Fatal("a second reader blocked on a key already held for reading")
	}
}

// TestWriteBeatsRead: taking both modes on one key would self-deadlock,
// because sync.RWMutex is not reentrant.
func TestWriteBeatsRead(t *testing.T) {
	locker := newBuildResourceLocker()

	done := make(chan struct{})
	go func() {
		release := locker.Acquire(
			[]string{"typescript/state-machine"},
			[]string{"typescript/state-machine", "typescript/directed-graph"},
		)
		release()
		close(done)
	}()

	select {
	case <-done:
	case <-time.After(2 * time.Second):
		t.Fatal("acquiring one key for both read and write deadlocked")
	}
}

// TestOverlappingKeySetsDoNotDeadlock exercises the sorted-acquisition
// invariant under -race with intersecting key sets.
func TestOverlappingKeySetsDoNotDeadlock(t *testing.T) {
	locker := newBuildResourceLocker()
	keys := []string{"a", "b", "c", "d", "e"}

	var wg sync.WaitGroup
	for i := 0; i < 40; i++ {
		wg.Add(1)
		go func(n int) {
			defer wg.Done()
			// Deliberately unsorted and overlapping, in varying order.
			writes := []string{keys[(n+2)%len(keys)], keys[n%len(keys)]}
			reads := []string{keys[(n+4)%len(keys)], keys[(n+1)%len(keys)]}
			release := locker.Acquire(writes, reads)
			release()
		}(i)
	}

	finished := make(chan struct{})
	go func() { wg.Wait(); close(finished) }()

	select {
	case <-finished:
	case <-time.After(10 * time.Second):
		t.Fatal("overlapping key sets deadlocked")
	}
}

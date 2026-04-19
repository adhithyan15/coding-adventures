package gitdiff

import (
	"os/exec"
	"sort"
	"strings"
)

// CIWorkflowPath is the main monorepo CI workflow whose changes can affect
// toolchain setup and build behavior.
const CIWorkflowPath = ".github/workflows/ci.yml"

// CIWorkflowChange describes how a ci.yml patch should influence change
// detection. Toolchain-scoped edits can be verified without forcing a full
// rebuild; shared edits must still fan out to the whole repo.
type CIWorkflowChange struct {
	Toolchains          map[string]bool
	RequiresFullRebuild bool
}

var ciWorkflowToolchainMarkers = map[string][]string{
	"python": {
		"needs_python", "setup-python", "python-version", "setup-uv",
		"python --version", "uv --version", "pytest",
		"set up python", "install uv",
	},
	"ruby": {
		"needs_ruby", "setup-ruby", "ruby-version", "bundler",
		"gem install bundler", "ruby --version", "bundle --version",
		"set up ruby", "install bundler",
	},
	"go": {
		"needs_go", "setup-go", "go-version", "go version", "set up go",
	},
	"typescript": {
		"needs_typescript", "setup-node", "node-version", "npm install -g jest",
		"node --version", "npm --version", "set up node",
	},
	"rust": {
		"needs_rust", "rust-toolchain", "cargo", "rustc", "tarpaulin",
		"wasm32-unknown-unknown", "set up rust", "install cargo-tarpaulin",
	},
	"elixir": {
		"needs_elixir", "setup-beam", "elixir-version", "otp-version",
		"elixir --version", "mix --version", "set up elixir",
	},
	"lua": {
		"needs_lua", "gh-actions-lua", "gh-actions-luarocks", "luarocks",
		"lua -v", "msvc", "set up lua", "set up luarocks",
	},
	"perl": {
		"needs_perl", "cpanm", "perl --version", "install cpanm",
	},
	"swift": {
		"needs_swift", "swift.toolchain", "winget install --id swift.toolchain",
		"swift --version", "swift -version", "set up swift", "swift.exe", "sdkroot",
		"-language swift", "swift packages",
	},
	"dart": {
		"needs_dart", "setup-dart", "dart --version", "set up dart",
	},
	"haskell": {
		"needs_haskell", "haskell-actions/setup", "ghc-version", "cabal-version",
		"ghc --version", "cabal --version", "set up haskell",
	},
	"java": {
		"needs_java", "setup-java", "java-version", "java --version",
		"temurin", "set up jdk", "set up gradle", "setup-gradle",
		"disable long-lived gradle services",
		"gradle_opts", "org.gradle.daemon", "org.gradle.vfs.watch",
	},
	"kotlin": {
		"needs_kotlin", "setup-java", "java-version",
		"temurin", "set up jdk", "set up gradle", "setup-gradle",
		"disable long-lived gradle services",
		"gradle_opts", "org.gradle.daemon", "org.gradle.vfs.watch",
	},
	"dotnet": {
		"needs_dotnet", "setup-dotnet", "dotnet-version", "dotnet --version",
		"set up .net",
	},
}

var ciWorkflowUnsafeMarkers = []string{
	"-detect-languages",
	"-emit-plan",
	"-force",
	"actions/checkout",
	"cancel-in-progress:",
	"concurrency:",
	"diff-base",
	"download-artifact",
	"event_name",
	"fetch-depth",
	"git fetch origin main",
	"git_ref",
	"is_main",
	"matrix:",
	"permissions:",
	"pr_base_ref",
	"pull_request:",
	"push:",
	"runs-on:",
	"strategy:",
	"upload-artifact",
}

// AnalyzeCIWorkflowChanges reads the current ci.yml patch against diffBase and
// classifies it as either toolchain-scoped or shared.
func AnalyzeCIWorkflowChanges(repoRoot, diffBase string) CIWorkflowChange {
	return AnalyzeCIWorkflowPatch(getFileDiff(repoRoot, diffBase, CIWorkflowPath))
}

// AnalyzeCIWorkflowPatch classifies a ci.yml patch. Conservative fallback:
// if a changed hunk touches shared CI behavior or cannot be tied to concrete
// toolchains, it requires a full rebuild.
func AnalyzeCIWorkflowPatch(patch string) CIWorkflowChange {
	change := CIWorkflowChange{
		Toolchains: make(map[string]bool),
	}

	var hunk []string
	flush := func() bool {
		toolchains, unsafe := classifyCIWorkflowHunk(hunk)
		if unsafe {
			change.RequiresFullRebuild = true
			change.Toolchains = nil
			return true
		}
		mergeToolchainSet(change.Toolchains, toolchains)
		hunk = hunk[:0]
		return false
	}

	for _, line := range strings.Split(patch, "\n") {
		switch {
		case strings.HasPrefix(line, "@@"):
			if flush() {
				return change
			}
		case strings.HasPrefix(line, "diff --git "),
			strings.HasPrefix(line, "index "),
			strings.HasPrefix(line, "--- "),
			strings.HasPrefix(line, "+++ "):
			continue
		default:
			hunk = append(hunk, line)
		}
	}

	flush()
	return change
}

func classifyCIWorkflowHunk(lines []string) (map[string]bool, bool) {
	hunkToolchains := make(map[string]bool)
	changedToolchains := make(map[string]bool)
	var changedLines []string

	for _, line := range lines {
		if len(line) == 0 {
			continue
		}
		if !isDiffLine(line) {
			continue
		}

		content := strings.TrimSpace(line[1:])
		mergeToolchainSet(hunkToolchains, detectCIWorkflowToolchains(content))

		if !isChangedLine(line) {
			continue
		}
		if content == "" || strings.HasPrefix(content, "#") {
			continue
		}
		changedLines = append(changedLines, content)
		mergeToolchainSet(changedToolchains, detectCIWorkflowToolchains(content))
	}

	if len(changedLines) == 0 {
		return nil, false
	}

	resolvedToolchains := changedToolchains
	if len(resolvedToolchains) == 0 {
		if len(hunkToolchains) != 1 {
			return nil, true
		}
		resolvedToolchains = hunkToolchains
	}

	for _, content := range changedLines {
		if lineTouchesSharedCIBehavior(content) {
			return nil, true
		}
		if len(detectCIWorkflowToolchains(content)) > 0 {
			continue
		}
		if isToolchainScopedStructuralLine(content) {
			continue
		}
		return nil, true
	}

	return resolvedToolchains, false
}

func detectCIWorkflowToolchains(content string) map[string]bool {
	found := make(map[string]bool)
	normalized := strings.ToLower(content)

	for toolchain, markers := range ciWorkflowToolchainMarkers {
		for _, marker := range markers {
			if strings.Contains(normalized, marker) {
				found[toolchain] = true
				break
			}
		}
	}

	return found
}

func lineTouchesSharedCIBehavior(content string) bool {
	normalized := strings.ToLower(content)
	for _, marker := range ciWorkflowUnsafeMarkers {
		if strings.Contains(normalized, marker) {
			return true
		}
	}
	return false
}

func isToolchainScopedStructuralLine(content string) bool {
	normalized := strings.ToLower(strings.TrimSpace(content))
	switch {
	case strings.HasPrefix(normalized, "if:"),
		strings.HasPrefix(normalized, "if ("),
		strings.HasPrefix(normalized, "if("),
		strings.HasPrefix(normalized, "run:"),
		strings.HasPrefix(normalized, "shell:"),
		strings.HasPrefix(normalized, "with:"),
		strings.HasPrefix(normalized, "env:"),
		strings.HasPrefix(normalized, "$"),
		strings.HasPrefix(normalized, "tmpdir=\"$(mktemp "),
		strings.HasPrefix(normalized, "("),
		strings.HasPrefix(normalized, ")"),
		strings.HasPrefix(normalized, "["),
		strings.HasPrefix(normalized, "{"),
		strings.HasPrefix(normalized, "}"),
		strings.HasPrefix(normalized, "else"),
		strings.HasPrefix(normalized, "fi"),
		strings.HasPrefix(normalized, "then"),
		strings.HasPrefix(normalized, "printf "),
		strings.HasPrefix(normalized, "echo "),
		strings.HasPrefix(normalized, "curl "),
		strings.HasPrefix(normalized, "sed -i.bak "),
		normalized == "rm -rf \"$tmpdir\"",
		strings.HasPrefix(normalized, "powershell "),
		strings.HasPrefix(normalized, "winget "),
		strings.HasPrefix(normalized, "foreach "),
		strings.HasPrefix(normalized, "where-object "),
		strings.HasPrefix(normalized, "sort-object "),
		strings.HasPrefix(normalized, "select-object "),
		strings.HasPrefix(normalized, "split-path "),
		strings.HasPrefix(normalized, "get-command "),
		strings.HasPrefix(normalized, "join-path "),
		strings.HasPrefix(normalized, "test-path "),
		strings.HasPrefix(normalized, "write-host "),
		strings.HasPrefix(normalized, "write-warning "),
		strings.HasPrefix(normalized, "write-output "),
		strings.HasPrefix(normalized, "where.exe "),
		strings.HasPrefix(normalized, "out-file "),
		strings.HasPrefix(normalized, "set-content "),
		strings.HasPrefix(normalized, "get-childitem "),
		strings.HasPrefix(normalized, "& "),
		strings.HasPrefix(normalized, "call "),
		strings.HasPrefix(normalized, "cd "):
		return true
	default:
		return false
	}
}

func isDiffLine(line string) bool {
	return strings.HasPrefix(line, " ") || isChangedLine(line)
}

func isChangedLine(line string) bool {
	return strings.HasPrefix(line, "+") || strings.HasPrefix(line, "-")
}

func mergeToolchainSet(dst, src map[string]bool) {
	for toolchain := range src {
		dst[toolchain] = true
	}
}

func getFileDiff(repoRoot, diffBase, relativePath string) string {
	cmd := exec.Command("git", "diff", "--unified=0", diffBase+"...HEAD", "--", relativePath)
	cmd.Dir = repoRoot
	out, err := cmd.Output()
	if err != nil {
		cmd = exec.Command("git", "diff", "--unified=0", diffBase, "HEAD", "--", relativePath)
		cmd.Dir = repoRoot
		out, err = cmd.Output()
		if err != nil {
			return ""
		}
	}
	return string(out)
}

// SortedToolchains returns the toolchain keys in a stable order for logging.
func SortedToolchains(toolchains map[string]bool) []string {
	var values []string
	for toolchain := range toolchains {
		values = append(values, toolchain)
	}
	sort.Strings(values)
	return values
}

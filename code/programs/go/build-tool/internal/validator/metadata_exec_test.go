package validator

import (
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestIsMetadataClassPath(t *testing.T) {
	for _, tc := range []struct {
		path string
		want bool
	}{
		{"evidence/run.sh", true},
		{"evidence/proofs/x.drat", true},
		{"./CHANGELOG.d/x.md", true},
		{"CHANGELOG.d/tool.py", true},
		{`..\pkg\CHANGELOG.md`, true},
		{"../dep/required_capabilities.json", true},
		{"effect_certificate.json", true},
		{"assurance_report.json", true},
		{"src/lib.rs", false},
		{"tools/run.sh", false},
		{"evidence", false},
		{"CHANGELOG.d", false},
		{"evidence.bindings", false},
		{"README.md", false},
		{"", false},
	} {
		if got := IsMetadataClassPath(tc.path); got != tc.want {
			t.Errorf("IsMetadataClassPath(%q) = %t, want %t", tc.path, got, tc.want)
		}
	}
}

// Each positive line is a way a BUILD or script could execute a metadata
// location; each negative line is a real data-only use from the repository.
func TestMetadataExecutionReferencesInCommands(t *testing.T) {
	for _, tc := range []struct {
		line string
		want []string
	}{
		{"sh evidence/run.sh", []string{"evidence/run.sh"}},
		{"bash -e ./CHANGELOG.d/hook.sh", []string{"./CHANGELOG.d/hook.sh"}},
		{". evidence/env.sh", []string{"evidence/env.sh"}},
		{"source CHANGELOG.d/x.md", []string{"CHANGELOG.d/x.md"}},
		{"python3 evidence/gen.py --out x", []string{"evidence/gen.py"}},
		{"FOO=1 node evidence/x.js", []string{"evidence/x.js"}},
		{"./evidence/run.sh", []string{"./evidence/run.sh"}},
		{"cargo test && sh evidence/run.sh", []string{"evidence/run.sh"}},
		{`powershell -File evidence\run.ps1`, []string{`evidence\run.ps1`}},
		{`call evidence\run.cmd`, []string{`evidence\run.cmd`}},
		{"& .\\evidence\\run.ps1", []string{`.\evidence\run.ps1`}},

		{"cargo run -p coding_adventures_ca_capability_analyzer -- --dir . --verify-cert effect_certificate.json", nil},
		{"WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p gf256 --test warrant_evidence", nil},
		{"set WARRANT_EVIDENCE_OUT=evidence/test-manifest.json&& cargo test -q -p gf256", nil},
		{"cargo run -p coding_adventures_ca_assurance_report -- --check assurance_report.json --allow-unverified", nil},
		{"cargo run -p required-capabilities-compiler -- --input ../weather-agent-e2e/required_capabilities.json", nil},
		{"# sh evidence/run.sh", nil},
		{"cargo test # sh evidence/run.sh", nil},
		{"echo see CHANGELOG.d/x.md", nil},
		{"sh tools/run.sh", nil},
	} {
		if got := MetadataExecutionReferences("code/packages/x/a/BUILD", tc.line); !reflect.DeepEqual(got, tc.want) {
			t.Errorf("%q → %v, want %v", tc.line, got, tc.want)
		}
	}
}

func TestMetadataExecutionReferencesInCargoAndRust(t *testing.T) {
	manifest := "[lib]\npath = \"evidence/lib.rs\"\n[[bin]]\nname = \"x\"\npath = \"CHANGELOG.d/main.rs\"\n" +
		"[package]\nbuild = \"evidence/build.rs\"\n[dependencies]\nok = { path = \"../ok\" }\n# path = \"evidence/commented.rs\"\n"
	if got, want := MetadataExecutionReferences("code/packages/rust/a/Cargo.toml", manifest), []string{"evidence/lib.rs", "CHANGELOG.d/main.rs", "evidence/build.rs"}; !reflect.DeepEqual(got, want) {
		t.Errorf("Cargo.toml → %v, want %v", got, want)
	}
	source := "#[path = \"../evidence/x.rs\"]\nmod x;\ninclude!(\"../CHANGELOG.d/gen.rs\");\n" +
		"const M: &str = include_str!(\"../required_capabilities.json\");\n" +
		"const B: &[u8] = include_bytes!(\"../evidence/test-manifest.json\");\n" +
		"// include!(\"../evidence/commented.rs\");\n#[path = \"imp/unix.rs\"]\nmod imp;\n"
	if got, want := MetadataExecutionReferences("code/packages/rust/a/src/lib.rs", source), []string{"../evidence/x.rs", "../CHANGELOG.d/gen.rs"}; !reflect.DeepEqual(got, want) {
		t.Errorf("lib.rs → %v, want %v", got, want)
	}
	if got := MetadataExecutionReferences("code/packages/rust/a/notes.txt", "sh evidence/run.sh"); got != nil {
		t.Errorf("an unscanned file type returned %v", got)
	}
}

// A real Git index: one violating BUILD, one data-only BUILD.
func TestValidateNoExecutedMetadataUsesTheIndex(t *testing.T) {
	root := t.TempDir()
	write := func(rel, text string) {
		t.Helper()
		full := filepath.Join(root, filepath.FromSlash(rel))
		if err := os.MkdirAll(filepath.Dir(full), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(full, []byte(text), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	git := func(args ...string) {
		t.Helper()
		cmd := exec.Command("git", args...)
		cmd.Dir = root
		if out, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v: %s", args, err, out)
		}
	}
	git("init", "-q")
	write("code/packages/rust/ok/BUILD", "cargo run -- --check assurance_report.json\n")
	git("add", ".")
	if err := ValidateNoExecutedMetadata(root); err != nil {
		t.Fatalf("data-only BUILD rejected: %v", err)
	}
	write("code/packages/rust/bad/BUILD", "sh evidence/run.sh\n")
	write("code/packages/rust/untracked/BUILD", "sh evidence/run.sh\n")
	git("add", "code/packages/rust/bad/BUILD")
	err := ValidateNoExecutedMetadata(root)
	if err == nil || !strings.Contains(err.Error(), "code/packages/rust/bad/BUILD") || strings.Contains(err.Error(), "untracked") {
		t.Fatalf("error = %v", err)
	}
}

// The checked-in repository must pass.
func TestRepositoryExecutesNoMetadata(t *testing.T) {
	// internal/validator → build-tool → go → programs → code → repository.
	root, err := filepath.Abs(filepath.Join("..", "..", "..", "..", "..", ".."))
	if err != nil {
		t.Fatal(err)
	}
	if _, statErr := os.Stat(filepath.Join(root, "code", "specs", "12-build-system.md")); statErr != nil {
		t.Fatalf("repository root not found at %s", root)
	}
	if err := ValidateNoExecutedMetadata(root); err != nil {
		t.Fatal(err)
	}
}

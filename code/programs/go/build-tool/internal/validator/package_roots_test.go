package validator

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
)

type orphanPackageFixture struct {
	Input struct {
		Options struct {
			Snapshot OrphanPackageRootSnapshot `json:"orphan_package_root_snapshot"`
		} `json:"options"`
	} `json:"input"`
	Expected struct {
		Result struct {
			Valid                 bool     `json:"valid"`
			DiagnosticCodes       []string `json:"diagnostic_codes"`
			PendingExemptionCount int      `json:"pending_exemption_count"`
		} `json:"result"`
		Diagnostics []OrphanPackageDiagnostic `json:"diagnostics"`
	} `json:"expected"`
}

func loadOrphanPackageFixture(t *testing.T, name string) orphanPackageFixture {
	t.Helper()
	_, sourceFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("could not locate package-root test source")
	}
	repoRoot := filepath.Clean(filepath.Join(filepath.Dir(sourceFile), "..", "..", "..", "..", "..", ".."))
	data, err := os.ReadFile(filepath.Join(repoRoot, "code", "specs", "fixtures", "build-tool-v1", "cases", name))
	if err != nil {
		t.Fatalf("read shared fixture %s: %v", name, err)
	}
	var fixture orphanPackageFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatalf("decode shared fixture %s: %v", name, err)
	}
	return fixture
}

func TestOrphanPackageRootValidationMatchesNeutralFixtures(t *testing.T) {
	cases := []string{
		"validation-orphan-package-roots-clean.json",
		"validation-orphan-package-roots-unlisted.json",
		"validation-orphan-package-root-exemptions-invalid.json",
		"validation-orphan-package-root-exemptions-stale.json",
	}
	for _, name := range cases {
		t.Run(name, func(t *testing.T) {
			fixture := loadOrphanPackageFixture(t, name)
			actual, err := ValidateOrphanPackageRootSnapshot(fixture.Input.Options.Snapshot)
			if err != nil {
				t.Fatalf("validate shared fixture: %v", err)
			}
			if actual.Valid != fixture.Expected.Result.Valid ||
				actual.PendingExemptionCount != fixture.Expected.Result.PendingExemptionCount ||
				!reflect.DeepEqual(actual.DiagnosticCodes, fixture.Expected.Result.DiagnosticCodes) ||
				!jsonEqual(actual.Diagnostics, fixture.Expected.Diagnostics) {
				actualJSON, _ := json.MarshalIndent(actual, "", "  ")
				expectedJSON, _ := json.MarshalIndent(fixture.Expected, "", "  ")
				t.Fatalf("neutral result mismatch\nactual: %s\nexpected: %s", actualJSON, expectedJSON)
			}
		})
	}
}

func TestOrphanPackageRootSnapshotPinsGeneratedRegistry(t *testing.T) {
	snapshot := OrphanPackageRootSnapshot{RegistrySHA256: strings.Repeat("0", 64)}
	if _, err := ValidateOrphanPackageRootSnapshot(snapshot); err == nil || !strings.Contains(err.Error(), "does not pin") {
		t.Fatalf("expected digest mismatch, got %v", err)
	}
	if hasher.SourceInputRegistryDigest() != "9bc672eac5d5ffc8e2d7d9ff94709a80bdcf0c2de01e7984d9cf8671d3dfa823" {
		t.Fatalf("unexpected generated registry digest %s", hasher.SourceInputRegistryDigest())
	}
}

func TestOrphanPackageRootSnapshotEnforcesCapsBeforeShape(t *testing.T) {
	snapshot := OrphanPackageRootSnapshot{
		RegistrySHA256: hasher.SourceInputRegistryDigest(),
		Roots:          make([]OrphanPackageRoot, maxOrphanPackageRoots+1),
	}
	if _, err := ValidateOrphanPackageRootSnapshot(snapshot); err == nil || !strings.Contains(err.Error(), "entry ceiling") {
		t.Fatalf("expected root cap failure, got %v", err)
	}
	snapshot.Roots = nil
	snapshot.Exemptions = make([]OrphanPackageExemption, 164)
	for index := range snapshot.Exemptions {
		snapshot.Exemptions[index] = OrphanPackageExemption{
			Line: index + 1, Kind: "PENDING", Path: "code/packages/python/demo",
			Reason: strings.Repeat("\u2028", 4096),
		}
	}
	if _, err := ValidateOrphanPackageRootSnapshot(snapshot); err == nil || !strings.Contains(err.Error(), "byte ceiling") {
		t.Fatalf("expected byte cap failure, got %v", err)
	}
	snapshot.Exemptions = make([]OrphanPackageExemption, maxOrphanPackageRoots+1)
	if _, err := ValidateOrphanPackageRootSnapshot(snapshot); err == nil || !strings.Contains(err.Error(), "entry ceiling") {
		t.Fatalf("expected exemption cap failure, got %v", err)
	}
}

func TestOrphanPackageRootSnapshotUsesCompactUTF8ByteCount(t *testing.T) {
	exemptions := make([]OrphanPackageExemption, 160)
	for index := range exemptions {
		exemptions[index] = OrphanPackageExemption{
			Line: index + 1, Kind: "PENDING", Path: "code/packages/python/demo",
			Reason: strings.Repeat("\u2028", 4096),
		}
	}
	snapshot := OrphanPackageRootSnapshot{
		RegistrySHA256: hasher.SourceInputRegistryDigest(),
		// Go's JSON encoder writes U+2028 as six ASCII bytes; the neutral
		// compact UTF-8 encoding uses three. Every reason also stays within
		// the schema's 4,096-scalar per-field ceiling.
		Exemptions: exemptions,
	}
	if _, err := ValidateOrphanPackageRootSnapshot(snapshot); err != nil {
		t.Fatalf("literal UTF-8 accounting should accept the snapshot: %v", err)
	}
}

func TestOrphanPackageRootSnapshotPreflightsFieldBounds(t *testing.T) {
	for name, reason := range map[string]string{
		"scalar ceiling": strings.Repeat("x", 4097),
		"invalid UTF-8":  string([]byte{0xff}),
	} {
		t.Run(name, func(t *testing.T) {
			snapshot := OrphanPackageRootSnapshot{
				RegistrySHA256: hasher.SourceInputRegistryDigest(),
				Exemptions: []OrphanPackageExemption{{
					Line: 1, Kind: "PENDING", Path: "code/packages/python/demo", Reason: reason,
				}},
			}
			if _, err := ValidateOrphanPackageRootSnapshot(snapshot); err == nil || !strings.Contains(err.Error(), "Unicode scalar ceiling") {
				t.Fatalf("expected bounded preflight failure, got %v", err)
			}
		})
	}
}

func TestNeutralCanonicalJSONLengthDistinguishesLiteralEscapes(t *testing.T) {
	encoded := []byte(`{"actual":"\u2028","literal":"\\u2028","mixed":"\\\\\u2029"}`)
	if got, want := neutralCanonicalJSONLength(encoded), len(encoded)-6; got != want {
		t.Fatalf("neutral canonical length = %d, want %d", got, want)
	}
}

func TestOrphanPackageRootSnapshotRejectsUnsafePortablePaths(t *testing.T) {
	for _, path := range []string{
		"code/packages/python/CON",
		"code/packages/python/" + strings.Repeat("x", 513),
	} {
		snapshot := OrphanPackageRootSnapshot{
			RegistrySHA256: hasher.SourceInputRegistryDigest(),
			Roots: []OrphanPackageRoot{{
				Path: path, Language: "python", Kind: "package", SourceEvidence: path + "/demo.py",
			}},
		}
		if _, err := ValidateOrphanPackageRootSnapshot(snapshot); err == nil {
			t.Fatalf("expected unsafe path %q to fail, got %v", path, err)
		}
	}
}

func TestNoOrphanPackageRootsScansDirectGovernedRoots(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "code/packages/python/covered/demo.py"), "VALUE = 1\n")
	writeFile(t, filepath.Join(root, "code/packages/python/BUILD"), "python -m pytest\n")
	writeFile(t, filepath.Join(root, "code/packages/python/target/generated.py"), "VALUE = 1\n")
	writeFile(t, filepath.Join(root, "code/packages/python/no-source/README.txt"), "not governed source\n")
	if err := ValidateNoOrphanPackageRoots(root); err != nil {
		t.Fatalf("ancestor BUILD and generated/no-source exclusions should pass: %v", err)
	}
}

func TestNoOrphanPackageRootsCodeBuildCoversEveryLane(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "code/packages/python/covered/demo.py"), "VALUE = 1\n")
	writeFile(t, filepath.Join(root, "code/BUILD"), "python -m pytest\n")
	if err := ValidateNoOrphanPackageRoots(root); err != nil {
		t.Fatalf("code/BUILD should cover a direct package root: %v", err)
	}
}

func TestPackageRootScannerRetainsOnlySmallestWitness(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "code/packages/python/demo/z.py"), "VALUE = 1\n")
	writeFile(t, filepath.Join(root, "code/packages/python/demo/a.py"), "VALUE = 1\n")
	snapshot, err := scanOrphanPackageRoots(root)
	if err != nil {
		t.Fatalf("scan direct roots: %v", err)
	}
	if len(snapshot.Roots) != 1 || snapshot.Roots[0].SourceEvidence != "code/packages/python/demo/a.py" {
		t.Fatalf("scanner retained %v, want only the smallest compact witness", snapshot.Roots)
	}
}

func TestNoOrphanPackageRootsReportsSiblingAndEmptyBuild(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "code/packages/python/empty/demo.py"), "VALUE = 1\n")
	writeFile(t, filepath.Join(root, "code/packages/python/empty/BUILD"), "# comment only\n")
	writeFile(t, filepath.Join(root, "code/packages/python/missing/demo.py"), "VALUE = 1\n")
	writeFile(t, filepath.Join(root, "code/packages/python/sibling/BUILD"), "python -m pytest\n")
	err := ValidateNoOrphanPackageRoots(root)
	if err == nil {
		t.Fatal("expected uncovered direct roots to fail")
	}
	for _, want := range []string{
		"ORPHAN_PACKAGE_ROOT_EMPTY_BUILD", "code/packages/python/empty",
		"ORPHAN_PACKAGE_ROOT_UNLISTED", "code/packages/python/missing",
	} {
		if !strings.Contains(err.Error(), want) {
			t.Errorf("error %q does not contain %q", err, want)
		}
	}
}

func TestNoOrphanPackageRootsSharesLedgerButIgnoresLegacyEntries(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "code/packages/python/pending/demo.py"), "VALUE = 1\n")
	writeFile(t, filepath.Join(root, "code/packages/rust/Cargo.toml"), "[workspace]\n")
	writeFile(t, filepath.Join(root, filepath.FromSlash(ExemptionsFile)), strings.Join([]string{
		"EXCLUDED code/packages/rust  # legacy virtual Cargo workspace",
		"PENDING code/packages/python/pending  # awaiting a BUILD front",
	}, "\n"))
	if err := ValidateNoOrphanPackageRoots(root); err != nil {
		t.Fatalf("direct-root exemption should pass and legacy len=3 entry should be ignored: %v", err)
	}
	if got := PendingOrphanPackageRootExemptionCount(root); got != 1 {
		t.Fatalf("package-root pending exemption count = %d, want 1", got)
	}
}

func TestNoOrphanPackageRootsRejectsMalformedLedger(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "code/packages/python/covered/demo.py"), "VALUE = 1\n")
	writeFile(t, filepath.Join(root, "code/packages/python/BUILD"), "python -m pytest\n")
	writeFile(t, filepath.Join(root, filepath.FromSlash(ExemptionsFile)), "UNKNOWN code/packages/python/covered  # invalid kind\n")
	if err := ValidateNoOrphanPackageRoots(root); err == nil || !strings.Contains(err.Error(), "invalid BUILD exemption ledger") {
		t.Fatalf("expected malformed ledger to fail closed, got %v", err)
	}
}

func TestRustVirtualWorkspaceClassificationIgnoresComments(t *testing.T) {
	manifest := "[workspace]\nmembers = []\n# [package] is not an active table\nnote = '''\n[package]\n'''\n"
	if !rustManifestIsVirtualWorkspace(manifest) {
		t.Fatal("commented package header must not convert a virtual workspace into a package")
	}
	if rustManifestIsVirtualWorkspace(manifest + "[package]\nname = \"real\"\n") {
		t.Fatal("active package header must make the root enforceable")
	}
}

func TestNoOrphanPackageRootsRejectsLinkedLane(t *testing.T) {
	root := t.TempDir()
	outside := t.TempDir()
	writeFile(t, filepath.Join(outside, "demo/demo.py"), "VALUE = 1\n")
	if err := os.MkdirAll(filepath.Join(root, "code/packages"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(outside, filepath.Join(root, "code/packages/python")); err != nil {
		t.Skipf("directory symlinks unavailable: %v", err)
	}
	if err := ValidateNoOrphanPackageRoots(root); err == nil || !strings.Contains(err.Error(), "directory chain") {
		t.Fatalf("expected linked lane to fail closed, got %v", err)
	}
}

func TestNoOrphanPackageRootsRejectsLinkedEvidenceAndBuilds(t *testing.T) {
	for _, target := range []string{"source", "build"} {
		t.Run(target, func(t *testing.T) {
			root := t.TempDir()
			outside := filepath.Join(t.TempDir(), "outside")
			if target == "source" {
				writeFile(t, outside+".py", "VALUE = 1\n")
				if err := os.MkdirAll(filepath.Join(root, "code/packages/python/demo"), 0o755); err != nil {
					t.Fatal(err)
				}
				if err := os.Symlink(outside+".py", filepath.Join(root, "code/packages/python/demo/demo.py")); err != nil {
					t.Skipf("symlinks unavailable: %v", err)
				}
			} else {
				writeFile(t, filepath.Join(root, "code/packages/python/demo/demo.py"), "VALUE = 1\n")
				writeFile(t, outside, "python -m pytest\n")
				if err := os.Symlink(outside, filepath.Join(root, "code/packages/python/BUILD")); err != nil {
					t.Skipf("symlinks unavailable: %v", err)
				}
			}
			if err := ValidateNoOrphanPackageRoots(root); err == nil || !strings.Contains(err.Error(), "linked or reparse-backed") {
				t.Fatalf("expected linked %s to fail closed, got %v", target, err)
			}
		})
	}
}

func TestSharedPendingReporterDoesNotDoubleCountRustRoot(t *testing.T) {
	root := t.TempDir()
	writeFile(t, filepath.Join(root, "code/packages/rust/demo/Cargo.toml"), "[package]\nname = \"demo\"\n")
	writeFile(t, filepath.Join(root, "code/packages/python/demo/demo.py"), "VALUE = 1\n")
	writeFile(t, filepath.Join(root, filepath.FromSlash(ExemptionsFile)), strings.Join([]string{
		"PENDING code/packages/rust/demo  # participates in both orphan gates",
		"PENDING code/packages/python/demo  # package-root-only backlog",
	}, "\n"))
	if err := ValidateNoOrphanCrates(root); err != nil {
		t.Fatalf("Cargo gate should accept its exemption: %v", err)
	}
	if err := ValidateNoOrphanPackageRoots(root); err != nil {
		t.Fatalf("package-root gate should accept both exemptions: %v", err)
	}
	if got := PendingExemptionCount(root); got != 2 {
		t.Fatalf("shared reporter = %d, want 2 unique ledger records", got)
	}
	if got := PendingOrphanPackageRootExemptionCount(root); got != 2 {
		t.Fatalf("package-root check count = %d, want 2", got)
	}
}

func TestNoOrphanPackageRootsRealRepoPasses(t *testing.T) {
	root, err := filepath.Abs(filepath.Join("..", "..", "..", "..", "..", ".."))
	if err != nil {
		t.Fatalf("resolving repo root: %v", err)
	}
	if _, statErr := os.Stat(filepath.Join(root, filepath.FromSlash("code/packages/go"))); statErr != nil {
		t.Skipf("repo layout not present at %s: %v", root, statErr)
	}
	if err := ValidateNoOrphanPackageRoots(root); err != nil {
		t.Fatalf("the repo does not pass its own orphan-package-root gate:\n%v", err)
	}
}

func jsonEqual(left, right any) bool {
	leftJSON, leftErr := json.Marshal(left)
	rightJSON, rightErr := json.Marshal(right)
	return leftErr == nil && rightErr == nil && string(leftJSON) == string(rightJSON)
}

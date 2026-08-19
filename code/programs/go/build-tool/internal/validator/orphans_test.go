package validator

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// crateSpec describes one crate to materialize in a temp repo.
type crateSpec struct {
	path      string // repo-relative, e.g. "code/packages/rust/foo"
	hasCargo  bool
	hasBuild  bool
	buildName string // defaults to "BUILD" when hasBuild is set
	// buildBody is the BUILD's contents. nil means "a real command"; a pointer
	// to "" means a genuinely empty file, which is the cheapest bypass of all
	// and therefore has to be expressible here.
	buildBody *string
}

func body(s string) *string { return &s }

// newRepo builds a throwaway repo tree. Everything the check looks at is on the
// filesystem, so the tests drive it the same way CI does — by making real files
// — rather than by injecting a fake.
func newRepo(t *testing.T, crates []crateSpec, ledger string) string {
	t.Helper()
	root := t.TempDir()

	for _, c := range crates {
		dir := filepath.Join(root, filepath.FromSlash(c.path))
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatalf("mkdir %s: %v", dir, err)
		}
		if c.hasCargo {
			writeFile(t, filepath.Join(dir, "Cargo.toml"), "[package]\nname = \"x\"\n")
		}
		if c.hasBuild {
			name := c.buildName
			if name == "" {
				name = "BUILD"
			}
			content := "cargo test -p x -- --nocapture\n"
			if c.buildBody != nil {
				content = *c.buildBody
			}
			writeFile(t, filepath.Join(dir, name), content)
		}
	}

	if ledger != "" {
		writeFile(t, filepath.Join(root, filepath.FromSlash(ExemptionsFile)), ledger)
	}
	return root
}

func writeFile(t *testing.T, path, content string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatalf("mkdir for %s: %v", path, err)
	}
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}

// A crate with a Cargo.toml and a BUILD is the healthy case and must pass.
func TestOrphans_CrateWithBuildPasses(t *testing.T) {
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: true},
	}, "")

	if err := ValidateNoOrphanCrates(root); err != nil {
		t.Fatalf("expected pass, got: %v", err)
	}
}

// The whole reason this check exists: a crate with no BUILD and no ledger entry
// must fail. This is the control — if it ever stops failing, the gate is dead.
func TestOrphans_UnlistedOrphanFails(t *testing.T) {
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: false},
	}, "")

	err := ValidateNoOrphanCrates(root)
	if err == nil {
		t.Fatal("expected an orphaned crate to fail validation, got nil")
	}
	if !strings.Contains(err.Error(), "code/packages/rust/alpha") {
		t.Errorf("error should name the offending crate, got: %v", err)
	}
	if !strings.Contains(err.Error(), ExemptionsFile) {
		t.Errorf("error should point at the ledger so the fix is obvious, got: %v", err)
	}
}

// Programs live in a second root and must be scanned too — a gap there is just
// as invisible as one under packages.
func TestOrphans_ProgramsRootIsScanned(t *testing.T) {
	root := newRepo(t, []crateSpec{
		{path: "code/programs/rust/tool", hasCargo: true, hasBuild: false},
	}, "")

	err := ValidateNoOrphanCrates(root)
	if err == nil || !strings.Contains(err.Error(), "code/programs/rust/tool") {
		t.Fatalf("expected the programs root to be scanned, got: %v", err)
	}
}

// A directory with no Cargo.toml is not a crate and must not be reported.
func TestOrphans_NonCrateDirectoryIgnored(t *testing.T) {
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/notacrate", hasCargo: false, hasBuild: false},
	}, "")

	if err := ValidateNoOrphanCrates(root); err != nil {
		t.Fatalf("a directory without Cargo.toml is not a crate, got: %v", err)
	}
}

// Both exemption kinds suppress the failure.
func TestOrphans_ExemptionsSuppress(t *testing.T) {
	for _, kind := range []string{"EXCLUDED", "PENDING"} {
		t.Run(kind, func(t *testing.T) {
			root := newRepo(t, []crateSpec{
				{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: false},
			}, kind+" code/packages/rust/alpha  # a stated reason\n")

			if err := ValidateNoOrphanCrates(root); err != nil {
				t.Fatalf("expected %s entry to suppress the failure, got: %v", kind, err)
			}
		})
	}
}

// An exemption with no reason is indistinguishable from an oversight, so it is
// rejected rather than honoured.
func TestOrphans_ExemptionWithoutReasonFails(t *testing.T) {
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: false},
	}, "PENDING code/packages/rust/alpha\n")

	err := ValidateNoOrphanCrates(root)
	if err == nil || !strings.Contains(err.Error(), "no reason") {
		t.Fatalf("expected a reasonless exemption to be rejected, got: %v", err)
	}
}

func TestOrphans_UnknownKindFails(t *testing.T) {
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: false},
	}, "MAYBE code/packages/rust/alpha  # hedging\n")

	err := ValidateNoOrphanCrates(root)
	if err == nil || !strings.Contains(err.Error(), "unknown kind") {
		t.Fatalf("expected an unknown kind to be rejected, got: %v", err)
	}
}

func TestOrphans_DuplicateEntryFails(t *testing.T) {
	ledger := "PENDING code/packages/rust/alpha  # first\n" +
		"PENDING code/packages/rust/alpha  # second\n"
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: false},
	}, ledger)

	err := ValidateNoOrphanCrates(root)
	if err == nil || !strings.Contains(err.Error(), "duplicate entry") {
		t.Fatalf("expected a duplicate entry to be rejected, got: %v", err)
	}
}

// The anti-rot rule. Once a crate gains a BUILD, its exemption must be deleted —
// otherwise the ledger becomes a place things are filed and forgotten.
func TestOrphans_StaleEntryForCrateThatNowHasBuildFails(t *testing.T) {
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: true},
	}, "PENDING code/packages/rust/alpha  # was a gap, now fixed\n")

	err := ValidateNoOrphanCrates(root)
	if err == nil {
		t.Fatal("expected a stale exemption to fail, got nil")
	}
	if !strings.Contains(err.Error(), "stale") || !strings.Contains(err.Error(), "now covered by a BUILD") {
		t.Errorf("error should explain the entry is stale, got: %v", err)
	}
}

// An entry naming a directory that no longer exists is also stale — this catches
// a crate that was deleted or renamed out from under the ledger.
func TestOrphans_StaleEntryForMissingDirectoryFails(t *testing.T) {
	root := newRepo(t, nil, "PENDING code/packages/rust/ghost  # long gone\n")

	err := ValidateNoOrphanCrates(root)
	if err == nil || !strings.Contains(err.Error(), "does not exist") {
		t.Fatalf("expected an entry for a missing directory to fail, got: %v", err)
	}
}

// Comments and blank lines are ignored, and CRLF ledgers parse identically —
// this repo is edited on Windows as well as Linux.
func TestOrphans_CommentsBlanksAndCRLF(t *testing.T) {
	ledger := "# a header comment\r\n\r\n" +
		"PENDING code/packages/rust/alpha  # a stated reason\r\n" +
		"   # an indented comment\r\n"
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: false},
	}, ledger)

	if err := ValidateNoOrphanCrates(root); err != nil {
		t.Fatalf("expected comments/blanks/CRLF to parse cleanly, got: %v", err)
	}
}

// One run should report every problem, not stop at the first — chasing errors
// one at a time is the slowest possible feedback loop.
func TestOrphans_ReportsAllProblemsAtOnce(t *testing.T) {
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: false},
		{path: "code/packages/rust/beta", hasCargo: true, hasBuild: false},
	}, "PENDING code/packages/rust/ghost  # long gone\n")

	err := ValidateNoOrphanCrates(root)
	if err == nil {
		t.Fatal("expected failures, got nil")
	}
	for _, want := range []string{"alpha", "beta", "ghost"} {
		if !strings.Contains(err.Error(), want) {
			t.Errorf("expected all problems in one report, %q missing from: %v", want, err)
		}
	}
}

// A repo with no ledger at all is fine as long as nothing is orphaned.
func TestOrphans_MissingLedgerIsFineWhenNothingIsOrphaned(t *testing.T) {
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: true},
	}, "")

	if err := ValidateNoOrphanCrates(root); err != nil {
		t.Fatalf("expected pass with no ledger and no orphans, got: %v", err)
	}
}

func TestPendingExemptionCount(t *testing.T) {
	ledger := "EXCLUDED code/packages/rust/alpha  # bridge crate\n" +
		"PENDING code/packages/rust/beta  # a gap\n" +
		"PENDING code/packages/rust/gamma  # another gap\n"
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/alpha", hasCargo: true},
		{path: "code/packages/rust/beta", hasCargo: true},
		{path: "code/packages/rust/gamma", hasCargo: true},
	}, ledger)

	if got := PendingExemptionCount(root); got != 2 {
		t.Errorf("PendingExemptionCount = %d, want 2 (EXCLUDED must not count)", got)
	}
}

// The real repo must pass its own gate. This is the test that would have caught
// the 84-crate gap, and it keeps the checked-in ledger honest: add a crate
// without a BUILD, or leave a stale entry behind, and this goes red.
func TestOrphans_RealRepoPasses(t *testing.T) {
	root, err := filepath.Abs(filepath.Join("..", "..", "..", "..", "..", ".."))
	if err != nil {
		t.Fatalf("resolving repo root: %v", err)
	}
	if _, statErr := os.Stat(filepath.Join(root, filepath.FromSlash("code/packages/rust"))); statErr != nil {
		t.Skipf("repo layout not present at %s: %v", root, statErr)
	}

	if err := ValidateNoOrphanCrates(root); err != nil {
		t.Fatalf("the repo does not pass its own orphan-crate gate:\n%v", err)
	}
}

// A crate nested inside another language's package is covered by that package's
// BUILD. This repo has ~170 such native-extension crates (for example
// code/packages/python/conduit/ext/conduit_native), and flagging them would turn
// the gate into noise that gets switched off.
func TestOrphans_AncestorBuildCovers(t *testing.T) {
	root := newRepo(t, []crateSpec{
		{path: "code/packages/python/conduit", hasBuild: true},
		{path: "code/packages/python/conduit/ext/conduit_native", hasCargo: true},
	}, "")

	if err := ValidateNoOrphanCrates(root); err != nil {
		t.Fatalf("an ancestor BUILD should cover a nested crate, got: %v", err)
	}
}

// ... but only a real ancestor. A sibling's BUILD must not launder a crate.
func TestOrphans_SiblingBuildDoesNotCover(t *testing.T) {
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/covered", hasCargo: true, hasBuild: true},
		{path: "code/packages/rust/naked", hasCargo: true},
	}, "")

	err := ValidateNoOrphanCrates(root)
	if err == nil || !strings.Contains(err.Error(), "naked") {
		t.Fatalf("a sibling BUILD must not cover another crate, got: %v", err)
	}
	if strings.Contains(err.Error(), "covered") {
		t.Errorf("the crate with its own BUILD should not be reported: %v", err)
	}
}

// `touch BUILD` must not satisfy the gate. An empty BUILD is accepted by
// discovery, yields an empty command list, and makes the package report success
// having compiled, tested and linted nothing — the cheapest possible bypass, and
// one that leaves no reviewable artifact behind.
func TestOrphans_EmptyBuildIsNotCoverage(t *testing.T) {
	for name, content := range map[string]string{
		"completely empty": "",
		"blank lines":      "\n\n   \n",
		"comments only":    "# TODO: write this\n# really\n",
	} {
		t.Run(name, func(t *testing.T) {
			root := newRepo(t, []crateSpec{
				{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: true, buildBody: body(content)},
			}, "")

			err := ValidateNoOrphanCrates(root)
			if err == nil {
				t.Fatal("expected an empty BUILD to be rejected, got nil")
			}
			if !strings.Contains(err.Error(), "no runnable commands") {
				t.Errorf("error should say the BUILD is empty, got: %v", err)
			}
		})
	}
}

// A platform-specific BUILD is still a BUILD. Discovery resolves BUILD_windows /
// BUILD_mac / BUILD_linux, so a crate shipping only one of those is genuinely
// built and must not be pushed into the ledger — a false EXCLUDED entry would
// put a lie in the file and weaken it permanently.
func TestOrphans_PlatformBuildCounts(t *testing.T) {
	for _, name := range []string{"BUILD_windows", "BUILD_mac", "BUILD_linux", "BUILD_mac_and_linux"} {
		t.Run(name, func(t *testing.T) {
			root := newRepo(t, []crateSpec{
				{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: true, buildName: name},
			}, "")

			if err := ValidateNoOrphanCrates(root); err != nil {
				t.Fatalf("%s should count as coverage, got: %v", name, err)
			}
		})
	}
}

// A ledger path must be repo-relative and inside the scanned tree.
// `filepath.Clean` does not strip a leading `..`, so an unchecked entry would be
// stat-ed outside the repo — a directory-existence oracle readable from the
// build status, and an entry that could never go stale because it can never
// become covered.
func TestOrphans_LedgerPathEscapesAreRejected(t *testing.T) {
	cases := map[string]string{
		"parent traversal": "../../../../Users/someone/.ssh",
		"absolute unix":    "/etc/passwd",
		"outside scanRoot": "notcode/packages/rust/alpha",
	}
	for name, badPath := range cases {
		t.Run(name, func(t *testing.T) {
			root := newRepo(t, []crateSpec{
				{path: "code/packages/rust/alpha", hasCargo: true},
			}, "EXCLUDED "+badPath+"  # a stated reason\n")

			err := ValidateNoOrphanCrates(root)
			if err == nil {
				t.Fatal("expected the escaping path to be rejected, got nil")
			}
			// The orphan itself must still be reported — a bad ledger line must
			// not suppress the finding it was aimed at.
			if !strings.Contains(err.Error(), "code/packages/rust/alpha") {
				t.Errorf("the real orphan should still be reported, got: %v", err)
			}
		})
	}
}

// Build artifacts and vendored sources hold Cargo.toml files that are not this
// repo's packages.
func TestOrphans_SkippedDirectoriesAreNotScanned(t *testing.T) {
	root := newRepo(t, []crateSpec{
		{path: "code/packages/rust/alpha", hasCargo: true, hasBuild: true},
		{path: "code/packages/rust/alpha/target/debug/somedep", hasCargo: true},
		{path: "code/packages/typescript/thing/node_modules/pkg", hasCargo: true},
	}, "")

	if err := ValidateNoOrphanCrates(root); err != nil {
		t.Fatalf("artifact directories must not be scanned, got: %v", err)
	}
}

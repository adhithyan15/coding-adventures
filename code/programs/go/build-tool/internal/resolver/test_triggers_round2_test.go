package resolver

// Regression tests for the round-2 security review of the Cargo trigger
// reader. Every test here failed against the round-1 reader (9bbf7fe0eee).

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// gitMarker makes the fixture root a repository root, so the reader finds
// code/BUILD-EXEMPTIONS beside it.
var gitMarker = map[string]string{".git/HEAD": "ref: refs/heads/main\n"}

func withFiles(base map[string]string, more map[string]string) map[string]string {
	merged := map[string]string{}
	for k, v := range base {
		merged[k] = v
	}
	for k, v := range more {
		merged[k] = v
	}
	return merged
}

// wantSubjectTriggersWith is wantSubjectTriggers with extra fixture files.
func wantSubjectTriggersWith(t *testing.T, subjectCargo string, extra map[string]string, want ...string) {
	t.Helper()
	_, triggers, err := ResolveWithTestTriggers(devDepWorldWith(t, subjectCargo, extra))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	var got []string
	for dep, dependents := range triggers {
		for _, dependent := range dependents {
			if dependent == "rust/subject" {
				got = append(got, dep)
			}
		}
	}
	if !sameSet(got, want) {
		t.Fatalf("rust/subject is triggered by %v, want %v", got, want)
	}
}

func wantRefusedWith(t *testing.T, subjectCargo string, extra map[string]string) {
	t.Helper()
	_, _, err := ResolveWithTestTriggers(devDepWorldWith(t, subjectCargo, extra))
	if err == nil {
		t.Fatal("want an error, got none")
	}
	if _, ok := err.(*CargoManifestError); !ok {
		t.Fatalf("want *CargoManifestError, got %T: %v", err, err)
	}
}

const devDepOn = plainSubject + "\n[dev-dependencies]\n"

// Finding A: a crate that is not a discovered package root is not a dead
// end — its own [dependencies] are linked into the dependent's tests.
func TestSecurityRound2HelperInsideTheDependentIsFollowed(t *testing.T) {
	wantSubjectTriggersWith(t, devDepOn+`h = { path = "helper" }`+"\n", map[string]string{
		"code/packages/rust/subject/helper/Cargo.toml": "[package]\nname = \"h\"\n\n[dependencies]\ncompiler = { path = \"../../compiler\" }\n",
	}, "rust/compiler", "rust/emit")
}

func TestSecurityRound2HelperInsideAnotherPackageIsFollowed(t *testing.T) {
	wantSubjectTriggersWith(t, devDepOn+`inner = { path = "../emit/inner" }`+"\n", map[string]string{
		"code/packages/rust/emit/inner/Cargo.toml": "[package]\nname = \"inner\"\n\n[build-dependencies]\nhelper-crate = { path = \"../../helper\" }\n",
	}, "rust/emit", "rust/helper")
}

func TestSecurityRound2ListedOrphanIsFollowed(t *testing.T) {
	wantSubjectTriggersWith(t, devDepOn+`orphan = { path = "../orphan" }`+"\n", withFiles(gitMarker, map[string]string{
		"code/BUILD-EXEMPTIONS": "EXCLUDED code/packages/rust/orphan  # test\n",
		"code/packages/rust/orphan/Cargo.toml": "[package]\nname = \"orphan\"\n\n" +
			"[target.'cfg(unix)'.dependencies]\ncompiler = { path = \"../compiler\" }\n\n" +
			// An orphan's own dev-dependencies are not linked into the
			// dependent's tests, so they are not followed.
			"[dev-dependencies]\nhelper-crate = { path = \"../helper\" }\n",
	}), "rust/compiler", "rust/emit")
}

func TestSecurityRound2UnlistedOrphansAreRefused(t *testing.T) {
	crate := "[package]\nname = \"x\"\n\n[dependencies]\ncompiler = { path = \"%s\" }\n"
	for name, tc := range map[string]struct{ dir, back string }{
		"vendor directory": {"code/packages/rust/vendor/launder", "../../compiler"},
		"outside code":     {"elsewhere/launder", "../../code/packages/rust/compiler"},
		"unlisted orphan":  {"code/packages/rust/orphan", "../compiler"},
		"listed by a typo": {"code/packages/rust/orphan2", "../compiler"},
	} {
		t.Run(name, func(t *testing.T) {
			rel, _ := filepath.Rel("code/packages/rust/subject", tc.dir)
			wantRefusedWith(t, devDepOn+fmt.Sprintf("x = { path = %q }\n", filepath.ToSlash(rel)), withFiles(gitMarker, map[string]string{
				"code/BUILD-EXEMPTIONS": "PENDING code/packages/rust/orphan22  # not an exact match\n",
				tc.dir + "/Cargo.toml":  fmt.Sprintf(crate, tc.back),
			}))
		})
	}
}

func TestSecurityRound2OrphanCyclesAreRefusedAndDepthIsCapped(t *testing.T) {
	// Round 3: a cycle among [dependencies] is refused (Cargo rejects it too),
	// so that a cached closure can never be one the cycle cut short.
	wantRefusedWith(t, devDepOn+`a = { path = "../orphan-a" }`+"\n", withFiles(gitMarker, map[string]string{
		"code/BUILD-EXEMPTIONS":                  "PENDING code/packages/rust/orphan-a  # t\nPENDING code/packages/rust/orphan-b  # t\n",
		"code/packages/rust/orphan-a/Cargo.toml": "[package]\nname = \"a\"\n[dependencies]\nb = { path = \"../orphan-b\" }\n",
		"code/packages/rust/orphan-b/Cargo.toml": "[package]\nname = \"b\"\n[dependencies]\na = { path = \"../orphan-a\" }\ncompiler = { path = \"../compiler\" }\n",
	}))

	// A chain longer than the cap is refused rather than truncated.
	files := withFiles(gitMarker, nil)
	var ledger strings.Builder
	for i := 0; i <= maxCargoPathDepth+1; i++ {
		dir := fmt.Sprintf("code/packages/rust/chain-%02d", i)
		ledger.WriteString("PENDING " + dir + "  # t\n")
		files[dir+"/Cargo.toml"] = fmt.Sprintf("[package]\nname = \"c%d\"\n[dependencies]\nnext = { path = \"../chain-%02d\" }\n", i, i+1)
	}
	files["code/BUILD-EXEMPTIONS"] = ledger.String()
	wantRefusedWith(t, devDepOn+`c = { path = "../chain-00" }`+"\n", files)
}

// Finding B: a symlinked directory resolves to its real package, and a path
// that names a package with the wrong letter case is refused.
func TestSecurityRound2SymlinkedDirectoryResolvesToItsTarget(t *testing.T) {
	packages := devDepWorld(t, devDepOn+`compiler = { path = "../compiler-link" }`+"\n")
	link := filepath.Join(filepath.Dir(packages[1].Path), "compiler-link")
	if err := os.Symlink(packages[1].Path, link); err != nil {
		t.Skipf("symlinks unavailable: %v", err)
	}
	_, triggers, err := ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatal(err)
	}
	if got := triggers.Dependents("rust/compiler"); !sameSet(got, []string{"mosaic/pkg-a", "rust/subject"}) {
		t.Fatalf("rust/compiler dependents = %v", got)
	}
}

func TestSecurityRound2CaseMismatchedPathIsRefused(t *testing.T) {
	packages := devDepWorld(t, devDepOn+`compiler = { path = "../Compiler" }`+"\n")
	_, _, err := ResolveWithTestTriggers(packages)
	if err == nil {
		t.Fatal("want an exact-spelling error, got none")
	}
	if !strings.Contains(err.Error(), `path dependency "../Compiler" does not use the directory's exact spelling`) {
		t.Fatalf("wrong error: %v", err)
	}
}

// Finding C: [patch.<source>] path entries are triggers; every other
// spelling of patch, and [replace], is refused.
func TestSecurityRound2PatchTablesAreTriggers(t *testing.T) {
	for _, header := range []string{`[patch.crates-io]`, `["patch"."crates-io"]`, `[patch.'https://github.com/x/y']`} {
		t.Run(header, func(t *testing.T) {
			wantSubjectTriggers(t, devDepOn+"zz = \"1\"\n\n"+header+"\nzz = { path = \"../compiler\" }\n", "rust/compiler", "rust/emit")
		})
	}
	for name, cargo := range map[string]string{
		"patch sub-table":      "[patch.crates-io.zz]\npath = \"../compiler\"\n" + plainSubject,
		"bare patch table":     "[patch]\ncrates-io = { zz = { path = \"../compiler\" } }\n" + plainSubject,
		"dotted root key":      "patch.crates-io.zz = { path = \"../compiler\" }\n" + plainSubject,
		"inline root key":      "patch = { crates-io = { zz = { path = \"../compiler\" } } }\n" + plainSubject,
		"replace table":        plainSubject + "\n[replace]\n\"zz:1.0.0\" = { path = \"../compiler\" }\n",
		"replace inline":       "replace = { \"zz:1.0.0\" = { path = \"../compiler\" } }\n" + plainSubject,
		"replace quoted table": plainSubject + "\n[\"replace\"]\n\"zz:1.0.0\" = { path = \"../compiler\" }\n",
	} {
		t.Run(name, func(t *testing.T) { wantRefused(t, cargo) })
	}
}

// Finding C: a Cargo configuration file that can redirect dependencies
// (`paths`, `[patch]`) anywhere from the package up to the repository root
// is refused; one that only sets a linker passes.
func TestSecurityRound2CargoConfigOverridesAreRefused(t *testing.T) {
	for name, files := range map[string]map[string]string{
		"paths in the package":          {"code/packages/rust/subject/.cargo/config.toml": "paths = [\"../compiler\"]\n"},
		"patch in an ancestor (legacy)": {"code/.cargo/config": "[patch.crates-io]\nzz = { path = \"packages/rust/compiler\" }\n"},
		"patch at the repository root":  {".cargo/config.toml": "[\"patch\".crates-io]\nzz = { path = \"x\" }\n"},
		"dotted patch key":              {"code/packages/.cargo/config.toml": "patch.crates-io.zz = { path = \"x\" }\n"},
		"undecodable config":            {"code/packages/.cargo/config.toml": "[pat\\u0063h]\n"},
	} {
		t.Run(name, func(t *testing.T) {
			wantRefusedWith(t, devDepOn+"zz = \"1\"\n", withFiles(gitMarker, files))
		})
	}
	// The repository's real configs only pick a Windows linker.
	wantSubjectTriggersWith(t, devDepOn+`compiler = { path = "../compiler" }`+"\n", withFiles(gitMarker, map[string]string{
		".cargo/config.toml":                    "[target.x86_64-pc-windows-msvc]\nlinker = \"rust-lld\"\n",
		"code/packages/rust/.cargo/config.toml": "# comment mentioning patch and paths\n[target.x86_64-pc-windows-msvc]\nlinker = \"rust-lld\"\n",
	}), "rust/compiler", "rust/emit")
}

// Finding D: continuation lines are joined once, and one entry is capped.
func TestSecurityRound2LongContinuedEntriesAreRefusedQuickly(t *testing.T) {
	var body strings.Builder
	body.WriteString(devDepOn + "compiler = { path = \"../compiler\", features = [\n")
	for body.Len() < maxCargoManifestBytes-64 {
		body.WriteString("\"a\",\n")
	}
	body.WriteString("] }\n")
	start := time.Now()
	wantRefused(t, body.String())
	if elapsed := time.Since(start); elapsed > time.Second {
		t.Fatalf("a 1 MiB continued entry took %v", elapsed)
	}

	// Over the line cap but well under the byte cap.
	var lines strings.Builder
	lines.WriteString(devDepOn + "compiler = { path = \"../compiler\", features = [\n")
	for i := 0; i <= maxCargoEntryLines; i++ {
		lines.WriteString("\"a\",\n")
	}
	lines.WriteString("] }\n")
	wantRefused(t, lines.String())
}

// A -language run resolves path entries against every discovered package:
// with only the filtered list, a path into a filtered-out package would look
// like an unlisted crate and stop the plan.
func TestSecurityRound2FilteredRunsResolveAgainstEveryPackage(t *testing.T) {
	all := devDepWorld(t, plainSubject)
	mosaicOnly := []discovery.Package{all[3]}
	if _, _, err := ResolveWithTestTriggers(mosaicOnly); err == nil {
		t.Fatal("without the full package list the compiler path should be refused")
	}
	_, triggers, err := ResolveWithTestTriggersAgainst(mosaicOnly, all)
	if err != nil {
		t.Fatal(err)
	}
	if got := triggers.Dependents("rust/compiler"); !sameSet(got, []string{"mosaic/pkg-a"}) {
		t.Fatalf("rust/compiler dependents = %v", got)
	}
}

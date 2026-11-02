package resolver

// Regression tests for the round-3 security review of the Cargo trigger
// reader. Every test here failed against the round-2 reader (b2eec645422).

import (
	"path/filepath"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// Finding 1: a discovered package root does not end the chain. Its
// target-specific and build dependencies are not build-graph edges, yet they
// are linked into the dependent's tests.
func TestSecurityRound3PackageRootDependenciesAreFollowed(t *testing.T) {
	wantSubjectTriggersWith(t, devDepOn+`helper-crate = { path = "../helper" }`+"\n", map[string]string{
		"code/packages/rust/helper/Cargo.toml": "[package]\nname = \"helper-crate\"\n\n" +
			"[target.'cfg(unix)'.dependencies]\ncompiler = { path = \"../compiler\" }\n\n" +
			"[build-dependencies]\nemit = { path = \"../emit\" }\n\n" +
			// The package's own dev-dependencies build only its own tests.
			"[dev-dependencies]\nmosaic-pkg-a = { path = \"../../mosaic/pkg-a\" }\n",
	}, "rust/helper", "rust/compiler", "rust/emit")
}

// Finding 1: a package root in another language whose Cargo.toml (a native
// extension) links repository crates.
func TestSecurityRound3ForeignPackageRootCrateIsFollowed(t *testing.T) {
	packages := devDepWorldWith(t, devDepOn+`native = { path = "../../python/native" }`+"\n", map[string]string{
		"code/packages/python/native/Cargo.toml": "[package]\nname = \"native\"\n\n[dependencies]\ncompiler = { path = \"../../rust/compiler\" }\n",
	})
	root := filepath.Dir(filepath.Dir(filepath.Dir(filepath.Dir(packages[0].Path))))
	packages = append(packages, discovery.Package{
		Name: "python/native", Path: filepath.Join(root, "code/packages/python/native"), Language: "python",
	})
	_, triggers, err := ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatal(err)
	}
	for _, dep := range []string{"python/native", "rust/compiler"} {
		found := false
		for _, dependent := range triggers.Dependents(dep) {
			found = found || dependent == "rust/subject"
		}
		if !found {
			t.Fatalf("%s should trigger rust/subject; triggers = %v", dep, triggers)
		}
	}
}

// Finding 1: transitive chains through package roots are followed, a
// diamond is read once, and a cycle among [dependencies] (which Cargo
// rejects) is refused rather than silently cut.
func TestSecurityRound3TransitiveChainsAndCycles(t *testing.T) {
	wantSubjectTriggersWith(t, devDepOn+`helper-crate = { path = "../helper" }`+"\n", map[string]string{
		"code/packages/rust/helper/Cargo.toml": "[package]\nname = \"helper-crate\"\n[dependencies]\n" +
			"emit = { path = \"../emit\" }\ncompiler = { path = \"../compiler\" }\n",
		"code/packages/rust/emit/Cargo.toml": "[package]\nname = \"emit\"\n" +
			"[target.'cfg(windows)'.dependencies]\npkg-a = { path = \"../../mosaic/pkg-a\" }\n",
	}, "rust/helper", "rust/emit", "rust/compiler", "mosaic/pkg-a")

	wantRefusedWith(t, devDepOn+`helper-crate = { path = "../helper" }`+"\n", map[string]string{
		"code/packages/rust/helper/Cargo.toml": "[package]\nname = \"helper-crate\"\n[dependencies]\nemit = { path = \"../emit\" }\n",
		"code/packages/rust/emit/Cargo.toml":   "[package]\nname = \"emit\"\n[build-dependencies]\nhelper-crate = { path = \"../helper\" }\n",
	})
}

// Finding 2: Cargo source replacement can redirect a registry dependency to
// repository files, so a config that declares [source] is refused.
func TestSecurityRound3CargoSourceReplacementIsRefused(t *testing.T) {
	for name, config := range map[string]string{
		"replace-with and directory": "[source.crates-io]\nreplace-with = \"vendored\"\n\n[source.vendored]\ndirectory = \"vendor\"\n",
		"dotted key":                 "source.crates-io.replace-with = \"vendored\"\n",
		"quoted header":              "[\"source\".vendored]\ndirectory = \"vendor\"\n",
	} {
		t.Run(name, func(t *testing.T) {
			wantRefusedWith(t, devDepOn+"zz = \"1\"\n", withFiles(gitMarker, map[string]string{
				"code/packages/rust/subject/.cargo/config.toml": config,
			}))
		})
	}
}

// Finding 3: a git dependency must name a remote; a local URL could point at
// a predictable checkout path and is linked with no trigger.
func TestSecurityRound3LocalGitDependenciesAreRefused(t *testing.T) {
	for name, entry := range map[string]string{
		"file url":      `zz = { git = "file:///home/runner/work/repo/code/packages/rust/compiler" }`,
		"absolute path": `zz = { git = "/tmp/compiler" }`,
		"relative path": `zz = { git = "../compiler" }`,
		"git protocol":  `zz = { git = "git://example.invalid/x" }`,
		"http":          `zz = { git = "http://example.invalid/x" }`,
		"in a patch":    "zz = \"1\"\n\n[patch.crates-io]\nzz = { git = \"file:///tmp/x\" }",
	} {
		t.Run(name, func(t *testing.T) { wantRefused(t, devDepOn+entry+"\n") })
	}
	wantSubjectTriggers(t, devDepOn+
		"a = { git = \"https://github.com/x/a\", rev = \"abc\" }\n"+
		"b = { git = \"ssh://git@github.com/x/b\" }\n")
}

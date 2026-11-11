package pkgmeta

import "testing"

// Every row is a package-relative Git path and whether it is package-local
// metadata. The false rows are the lookalikes: each one is read, could be
// read, or could be executed by something other than its own package's BUILD.
func TestIsPackageLocal(t *testing.T) {
	for _, tc := range []struct {
		path string
		want bool
	}{
		{"CHANGELOG.md", true},
		{"CHANGELOG.d/_meta.md", true},
		{"CHANGELOG.d/2026-11-11-0123abcd-x.md", true},
		{"required_capabilities.json", true},
		{"effect_certificate.json", true},
		{"assurance_report.json", true},
		{"evidence/test-manifest.json", true},
		{"evidence/proofs/abc.cnf", true},
		{"evidence/proofs/abc.drat", true},

		// README.md is read by dependents' packaging (hatchling readme).
		{"README.md", false},

		{"", false},
		{"src/lib.rs", false},
		{"Cargo.toml", false},
		{"BUILD", false},
		{"BUILD_windows", false},
		{"docs/README.md", false},
		{"sub/required_capabilities.json", false},
		{"x/CHANGELOG.d/y.md", false},
		{"x/evidence/y.json", false},
		{"changelog.md", false},
		{"Changelog.md", false},
		{"CHANGELOG", false},
		{"CHANGELOG.md.bak", false},
		{"Required_capabilities.json", false},
		{"required_capabilities.json.orig", false},
		{"Evidence/test-manifest.json", false},
		{"CHANGELOG.D/a.md", false},

		// Closed by use: only direct markdown shards, only evidence data.
		{"CHANGELOG.d", false},
		{"CHANGELOG.d/", false},
		{"CHANGELOG.d/.md", false},
		{"CHANGELOG.d/nested/deeper.md", false},
		{"CHANGELOG.d/run.sh", false},
		{"CHANGELOG.d/a.md.sh", false},
		{"CHANGELOG.d/a.rs", false},
		{"evidence", false},
		{"evidence/", false},
		{"evidence/.json", false},
		{"evidence/run.sh", false},
		{"evidence/lib.rs", false},
		{"evidence/x.json.sh", false},
		{"evidence/x.py", false},
		{"evidence/README.md", false},
		{"evidence//x.json", false},
		{"evidence/../src/x.json", false},
		{"evidence.bindings", false},
		{"evidence-old/x.json", false},
		{"CHANGELOG.dx/a.md", false},

		// Backslash spellings are not Git paths.
		{"CHANGELOG.d\\a.md", false},
		{"evidence\\test-manifest.json", false},
	} {
		if got := IsPackageLocal(tc.path); got != tc.want {
			t.Errorf("IsPackageLocal(%q) = %t, want %t", tc.path, got, tc.want)
		}
	}
}

package pkgmeta

import "testing"

// Every row is a package-relative Git path and whether it is package-local
// metadata. The false rows are the lookalikes: each one is read, or could be
// read, by something other than its own package's BUILD.
func TestIsPackageLocal(t *testing.T) {
	for _, tc := range []struct {
		path string
		want bool
	}{
		{"README.md", true},
		{"CHANGELOG.md", true},
		{"CHANGELOG.d/_meta.md", true},
		{"CHANGELOG.d/2026-11-11-0123abcd-x.md", true},
		{"CHANGELOG.d/nested/deeper.md", true},
		{"required_capabilities.json", true},
		{"effect_certificate.json", true},
		{"assurance_report.json", true},
		{"evidence/test-manifest.json", true},
		{"evidence/proofs/abc.drat", true},

		{"", false},
		{"src/lib.rs", false},
		{"Cargo.toml", false},
		{"BUILD", false},
		{"docs/README.md", false},
		{"tests/fixtures/README.md", false},
		{"sub/required_capabilities.json", false},
		{"readme.md", false},
		{"Readme.md", false},
		{"README", false},
		{"README.md.bak", false},
		{"CHANGELOG", false},
		{"CHANGELOG.d", false},
		{"CHANGELOG.d/", false},
		{"evidence", false},
		{"evidence/", false},
		{"evidence.bindings", false},
		{"evidence-old/x.json", false},
		{"CHANGELOG.dx/a.md", false},
		{"required_capabilities.json.orig", false},
		{"CHANGELOG.d\\a.md", false},
		{"evidence\\test-manifest.json", false},
	} {
		if got := IsPackageLocal(tc.path); got != tc.want {
			t.Errorf("IsPackageLocal(%q) = %t, want %t", tc.path, got, tc.want)
		}
	}
}

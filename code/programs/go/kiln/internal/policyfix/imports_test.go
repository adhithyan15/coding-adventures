package policyfix

import (
	"os/exec"
	"strings"
	"testing"
)

// policyfix is test support and panics outside a source tree, so no production
// package may import it.  `go list` reports the non-test imports of every
// package (test files are listed separately as TestImports), so any line that
// names policyfix here is a production import.
func TestNoProductionPackageImportsPolicyfix(t *testing.T) {
	goBin, err := exec.LookPath("go")
	if err != nil {
		t.Skip("go is not on PATH")
	}
	out, err := exec.Command(goBin, "list", "-f", "{{.ImportPath}} {{.Imports}}", "../../...").CombinedOutput()
	if err != nil {
		t.Skipf("go list failed (no module context?): %v\n%s", err, out)
	}
	for _, line := range strings.Split(string(out), "\n") {
		path, imports, _ := strings.Cut(line, " ")
		if path == "" || strings.HasSuffix(path, "/internal/policyfix") {
			continue
		}
		if strings.Contains(imports, "/internal/policyfix") {
			t.Errorf("production package %s imports internal/policyfix", path)
		}
	}
}

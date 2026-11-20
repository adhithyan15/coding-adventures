package validator

// Users of the negative-control checker declare Python
// ====================================================
//
// code/scripts/warrant_negctl.py is the shared negative-control checker. BUILD
// files run it as `python3 ...` (`python ...` on Windows), but CI installs
// Python for a package only if its BUILD declares `# needs-toolchain: python`.
// A BUILD that calls the checker without the declaration passes locally and
// fails on a runner that has no Python.
//
// This rule used to live only in a build-tool Go test
// (TestEveryBuildFileUsingTheNegctlCheckerDeclaresPython). Since BUILD-front
// edits in a Rust package are package-local, a pull request that touches only
// that package's BUILD never selects go/programs/build-tool, so the test never
// ran on the PR that broke it. -validate-build-files runs on every pull
// request, so the rule is enforced here.
//
// The rule is the old test's, unchanged:
//
//   - a BUILD file "uses" the checker if its text contains the substring
//     `warrant_negctl.py`. This is deliberately a plain substring match: a
//     mention in a comment counts (the safe direction: a false positive costs
//     one directive line, a false negative costs a red runner), and so does a
//     lookalike such as `warrant_negctl.py.bak`; `other_negctl.py` does not.
//   - every file whose name starts with `BUILD` in a package directory is
//     checked, so BUILD, BUILD_linux, BUILD_mac, BUILD_mac_and_linux and
//     BUILD_windows are all covered.
//   - the file "declares Python" if the platform it serves resolves to a
//     toolchain list containing `python`, through the same parser discovery
//     uses (discovery.ExtraToolchainsForSnapshot): `_windows` is windows,
//     `_mac` is darwin, anything else is linux.

import (
	"fmt"
	"os"
	"path/filepath"
	"slices"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// negctlCheckerScript is the substring that marks a BUILD file as a user.
const negctlCheckerScript = "warrant_negctl.py"

// ValidateNegctlUsersDeclarePython returns an error naming every BUILD file
// in the given packages that calls the negative-control checker without
// declaring `# needs-toolchain: python` for the platform it serves.
func ValidateNegctlUsersDeclarePython(packages []discovery.Package) error {
	var findings []string
	for _, pkg := range packages {
		entries, err := os.ReadDir(pkg.Path)
		if err != nil {
			continue
		}
		for _, entry := range entries {
			name := entry.Name()
			if entry.IsDir() || !strings.HasPrefix(name, "BUILD") {
				continue
			}
			data, err := os.ReadFile(filepath.Join(pkg.Path, name))
			if err != nil {
				continue
			}
			content := string(data)
			if !strings.Contains(content, negctlCheckerScript) {
				continue
			}
			goos := "linux"
			switch {
			case strings.HasSuffix(name, "_windows"):
				goos = "windows"
			case strings.HasSuffix(name, "_mac"):
				goos = "darwin"
			}
			declared := discovery.ExtraToolchainsForSnapshot(map[string]string{name: content}, goos)
			if !slices.Contains(declared, "python") {
				findings = append(findings, fmt.Sprintf("%s/%s (declares %v)", pkg.Name, name, declared))
			}
		}
	}
	if len(findings) == 0 {
		return nil
	}
	sort.Strings(findings)
	return fmt.Errorf(
		"negative-control checker validation failed: these BUILD files call %s but do not declare `# needs-toolchain: python`:\n  - %s\n"+
			"CI installs Python for a package only when its BUILD declares it; add a `# needs-toolchain: python` line to each file listed.",
		negctlCheckerScript, strings.Join(findings, "\n  - "),
	)
}

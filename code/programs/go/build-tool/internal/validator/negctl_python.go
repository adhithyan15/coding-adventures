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
//   - every file whose name starts with `BUILD` directly in a package
//     directory is checked, so BUILD, BUILD_linux, BUILD_mac,
//     BUILD_mac_and_linux and BUILD_windows are all covered. "Package
//     directory" means a directory discovered for ANY platform
//     (discovery.DiscoverPackagesAllPlatforms): CI's validation job runs on
//     Linux, where a directory whose only file is BUILD_windows is not a
//     discovered package, and a user added only there must still be caught.
//     Nested BUILD files that are not in a package directory are not checked:
//     the build tool never runs them.
//   - a file that cannot be read (unreadable, a dangling symlink, a BUILD*
//     that is a directory) is reported as a finding rather than skipped, so
//     the check fails closed. Only a package directory that does not exist
//     (plan mode may name packages absent from a sparse checkout) is skipped.
//   - the file "declares Python" if the platform it serves resolves to a
//     toolchain list containing `python`, through the same parser discovery
//     uses (discovery.ExtraToolchainsForSnapshot): `_windows` is windows,
//     `_mac` is darwin, anything else is linux.

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"slices"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// negctlCheckerScript is the substring that marks a BUILD file as a user.
const negctlCheckerScript = "warrant_negctl.py"

// negctlBuildFileNames are the names the build tool can run as a BUILD file.
// A flagged file outside this set (BUILD.bak, BUILD-notes.md) is never run.
var negctlBuildFileNames = []string{"BUILD", "BUILD_linux", "BUILD_mac", "BUILD_mac_and_linux", "BUILD_windows"}

// ValidateNegctlUsersDeclarePython returns an error naming every BUILD file
// in the given packages that calls the negative-control checker without
// declaring `# needs-toolchain: python` for the platform it serves, or that
// could not be read. Paths in the report are relative to repoRoot. Callers
// should pass the all-platforms package list.
func ValidateNegctlUsersDeclarePython(repoRoot string, packages []discovery.Package) error {
	var findings []string
	rel := func(path string) string {
		if r, err := filepath.Rel(repoRoot, path); err == nil {
			return filepath.ToSlash(r)
		}
		return filepath.ToSlash(path)
	}
	for _, pkg := range packages {
		entries, err := os.ReadDir(pkg.Path)
		if err != nil {
			if errors.Is(err, fs.ErrNotExist) {
				continue
			}
			findings = append(findings, fmt.Sprintf("%s (cannot read package directory: %v)", rel(pkg.Path), err))
			continue
		}
		for _, entry := range entries {
			name := entry.Name()
			if !strings.HasPrefix(name, "BUILD") {
				continue
			}
			path := filepath.Join(pkg.Path, name)
			data, err := os.ReadFile(path)
			if err != nil {
				findings = append(findings, fmt.Sprintf("%s (cannot read: %v)", rel(path), err))
				continue
			}
			content := string(data)
			if !strings.Contains(content, negctlCheckerScript) {
				continue
			}
			if !slices.Contains(negctlBuildFileNames, name) {
				findings = append(findings, fmt.Sprintf("%s (rename or remove it (the build tool never runs it))", rel(path)))
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
				findings = append(findings, fmt.Sprintf("%s (declares %v; add the directive)", rel(path), declared))
			}
		}
	}
	if len(findings) == 0 {
		return nil
	}
	sort.Strings(findings)
	return fmt.Errorf(
		"negative-control checker validation failed: these BUILD files call %s without declaring `# needs-toolchain: python`, or could not be read:\n  - %s\n"+
			"CI installs Python for a package only when its BUILD declares it; add a `# needs-toolchain: python` line to each build file listed, and fix or remove the others as noted.",
		negctlCheckerScript, strings.Join(findings, "\n  - "),
	)
}

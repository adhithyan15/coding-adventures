package validator

// Tracked-artifact validation.
//
// Dependency installation directories are machine-local build products. A
// tracked node_modules path can be especially deceptive when it is a symlink:
// the checkout works on its author's machine while clean worktrees and every
// other operating system receive a broken absolute link. Gitignore rules do
// not help once a path is already in the index, so this validation inspects the
// index itself.

import (
	"bytes"
	"fmt"
	"os/exec"
	"sort"
	"strings"
)

// ValidateNoTrackedNodeModules fails when any tracked path has a path component
// named node_modules. The check uses NUL-delimited index output so whitespace,
// tabs, and newlines in filenames cannot make a tracked artifact invisible.
func ValidateNoTrackedNodeModules(repoRoot string) error {
	cmd := exec.Command("git", "-C", repoRoot, "ls-files", "--stage", "-z")
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	output, err := cmd.Output()
	if err != nil {
		detail := strings.TrimSpace(stderr.String())
		if detail != "" {
			return fmt.Errorf("listing tracked files for node_modules validation: %w: %s", err, detail)
		}
		return fmt.Errorf("listing tracked files for node_modules validation: %w", err)
	}

	paths, err := trackedNodeModulesPaths(output)
	if err != nil {
		return fmt.Errorf("parsing tracked files for node_modules validation: %w", err)
	}
	if len(paths) == 0 {
		return nil
	}

	return fmt.Errorf(
		"tracked-artifact validation failed: node_modules is generated dependency state and must not be committed:\n  - %s\n"+
			"Remove these paths from the Git index; the repository .gitignore already excludes node_modules/.",
		strings.Join(paths, "\n  - "),
	)
}

func trackedNodeModulesPaths(output []byte) ([]string, error) {
	var paths []string
	for _, record := range bytes.Split(output, []byte{0}) {
		if len(record) == 0 {
			continue
		}
		tab := bytes.IndexByte(record, '\t')
		if tab < 0 || tab == len(record)-1 {
			return nil, fmt.Errorf("malformed git ls-files --stage record %q", record)
		}
		path := string(record[tab+1:])
		for _, component := range strings.Split(strings.ReplaceAll(path, `\`, "/"), "/") {
			if component == "node_modules" {
				paths = append(paths, path)
				break
			}
		}
	}
	sort.Strings(paths)
	return paths, nil
}

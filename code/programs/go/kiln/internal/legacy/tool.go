package legacy

import (
	"fmt"
	"path"
	"sort"
	"strings"
)

// goModReplaces returns the local (relative-path) targets of every `replace`
// directive in a go.mod.  Both forms are understood:
//
//	replace example.com/x => ../x
//	replace (
//	    example.com/y v1.2.3 => ./y
//	)
//
// A target is local when it is "." or ".." or begins with "./" or "../" (Go's
// own rule, also read with backslashes as separators); other targets are
// module paths and are not in the repository.  Comments (//) are
// stripped, and quoted targets are unquoted.  A target containing a backslash
// is refused rather than guessed at.
func goModReplaces(gomod string) ([]string, error) {
	var out []string
	inBlock := false
	for _, raw := range strings.Split(gomod, "\n") {
		line := raw
		if i := strings.Index(line, "//"); i >= 0 {
			line = line[:i]
		}
		line = strings.TrimSpace(line)
		switch {
		case line == "":
			continue
		case inBlock && line == ")":
			inBlock = false
			continue
		case !inBlock && (line == "replace (" || line == "replace("):
			inBlock = true
			continue
		case inBlock:
		case strings.HasPrefix(line, "replace ") || strings.HasPrefix(line, "replace\t"):
			line = strings.TrimSpace(line[len("replace"):])
		default:
			continue
		}
		_, rhs, ok := strings.Cut(line, "=>")
		if !ok {
			continue
		}
		f := strings.Fields(rhs)
		if len(f) == 0 {
			continue
		}
		target := strings.Trim(f[0], "\"`")
		norm := strings.ReplaceAll(target, "\\", "/")
		local := norm == "." || norm == ".." || strings.HasPrefix(norm, "./") || strings.HasPrefix(norm, "../")
		if !local {
			continue
		}
		if norm != target {
			return nil, fmt.Errorf("go.mod replace target %q contains a backslash", short(target))
		}
		out = append(out, target)
	}
	return out, nil
}

// goWorkDirs returns the directories a go.work names (relative to the
// directory holding it): every `use` target, in both forms,
//
//	use ./a
//	use (
//	    ./b
//	)
//
// plus the local targets of its `replace` directives.  Targets are returned
// exactly as written; the caller resolves and checks them.
func goWorkDirs(gowork string) ([]string, error) {
	var out []string
	inUse := false
	for _, raw := range strings.Split(gowork, "\n") {
		line := raw
		if i := strings.Index(line, "//"); i >= 0 {
			line = line[:i]
		}
		line = strings.TrimSpace(line)
		switch {
		case line == "":
		case inUse && line == ")":
			inUse = false
		case !inUse && (line == "use (" || line == "use("):
			inUse = true
		case inUse:
			out = append(out, unquote(line))
		case strings.HasPrefix(line, "use ") || strings.HasPrefix(line, "use\t"):
			out = append(out, unquote(strings.TrimSpace(line[len("use"):])))
		}
	}
	reps, err := goModReplaces(gowork)
	if err != nil {
		return nil, err
	}
	for _, d := range out {
		if strings.Contains(d, "\\") {
			return nil, fmt.Errorf("go.work use target %q contains a backslash", short(d))
		}
	}
	return append(out, reps...), nil
}

func unquote(s string) string { return strings.Trim(s, "\"`") }

// buildToolFiles lists the repository files that make up "the old build tool's
// source" for the tool:build-tool consumed entry (KILN03 3.4): every file of
// code/programs/go/build-tool, every file of each in-repo module its go.mod
// (transitively) replaces, every directory a root go.work `use`s or replaces
// (transitively), plus go.work, go.work.sum and vendor/ when present.  The
// result is sorted and unique.
//
// Fail closed: a missing build tool, a replace or use target that leaves the
// repository or names the repository root itself, and a backslash in a target
// are all errors, never silently skipped, because a skipped dependency would
// be a dependency the key cannot see.
func (pl *planner) buildToolFiles() ([]string, error) {
	dirs := []string{BuildToolPath}
	seen := map[string]bool{BuildToolPath: true}
	add := func(base, target, from string) error {
		dir := path.Join(base, target)
		if dir == "." || dir == ".." || strings.HasPrefix(dir, "../") {
			return fmt.Errorf("%s: target %q leaves the repository or names its root", from, short(target))
		}
		if !seen[dir] {
			seen[dir] = true
			dirs = append(dirs, dir)
		}
		return nil
	}
	if pl.fileSet["go.work"] {
		data, err := readSmall(pl.opt.Source, "go.work", maxPolicyBytes)
		if err != nil {
			return nil, err
		}
		targets, err := goWorkDirs(string(data))
		if err != nil {
			return nil, fmt.Errorf("go.work: %w", err)
		}
		for _, t := range targets {
			if err := add(".", t, "go.work"); err != nil {
				return nil, err
			}
		}
	}
	for i := 0; i < len(dirs); i++ {
		mod := dirs[i] + "/go.mod"
		if !pl.fileSet[mod] {
			if dirs[i] == BuildToolPath {
				return nil, fmt.Errorf("%s: not in the source tree", mod)
			}
			continue // a used or replaced directory without go.mod is still hashed
		}
		data, err := readSmall(pl.opt.Source, mod, maxPolicyBytes)
		if err != nil {
			return nil, err
		}
		targets, err := goModReplaces(string(data))
		if err != nil {
			return nil, fmt.Errorf("%s: %w", mod, err)
		}
		for _, t := range targets {
			if err := add(dirs[i], t, mod); err != nil {
				return nil, err
			}
		}
	}
	set := map[string]bool{}
	for _, d := range dirs {
		for _, f := range pl.under(d) {
			set[f] = true
		}
	}
	if len(pl.under(BuildToolPath)) == 0 {
		return nil, fmt.Errorf("%s: no files in the source tree", BuildToolPath)
	}
	for _, f := range []string{"go.work", "go.work.sum"} {
		if pl.fileSet[f] {
			set[f] = true
		}
	}
	for _, f := range pl.under("vendor") {
		set[f] = true
	}
	out := make([]string, 0, len(set))
	for f := range set {
		out = append(out, f)
	}
	sort.Strings(out)
	return out, nil
}

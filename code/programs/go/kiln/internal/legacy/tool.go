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

// buildToolFiles lists the repository files that make up "the old build tool's
// source" for the tool:build-tool consumed entry (KILN03 3.4): every file of
// code/programs/go/build-tool, every file of each in-repo module its go.mod
// (transitively) replaces, plus go.work, go.work.sum and vendor/ when present.
// The result is sorted and unique.  A missing build tool is an error, because
// an empty digest would make every key blind to the tool.
func (pl *planner) buildToolFiles() ([]string, error) {
	dirs := []string{BuildToolPath}
	seen := map[string]bool{BuildToolPath: true}
	for i := 0; i < len(dirs); i++ {
		mod := dirs[i] + "/go.mod"
		if !pl.fileSet[mod] {
			if i == 0 {
				return nil, fmt.Errorf("%s: not in the source tree", mod)
			}
			continue // a replaced directory without go.mod is still hashed
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
			dir := path.Join(dirs[i], t)
			if dir == ".." || strings.HasPrefix(dir, "../") || dir == "." || seen[dir] {
				continue // outside the repository (or the whole repository: refused)
			}
			seen[dir] = true
			dirs = append(dirs, dir)
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

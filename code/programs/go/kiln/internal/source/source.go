package source

import (
	"fmt"
	"io"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
)

// Source answers questions about the files being built (KILN00 section 13).
//
// Every path in and out is repository-relative, "/"-separated and valid per
// KILN02 section 3.5, whatever the operating system.
type Source interface {
	// Files lists the files under root (a repo-relative directory; "" or "."
	// means the whole tree) whose path RELATIVE TO root matches at least one
	// pattern (see Match; no patterns means all).  Results are repo-relative,
	// sorted bytewise, and exclude ignored files.  Symbolic links are listed
	// as entries and never followed.
	Files(root string, patterns []string) ([]string, error)
	// Open reads one regular file.  Symlinks are refused.
	Open(path string) (io.ReadCloser, error)
	// Revision is an opaque snapshot id.
	Revision() (string, error)
	// Changed lists paths changed since a revision.  known=false means the
	// source cannot say (not an error): planning proceeds from keys alone.
	Changed(since string) (paths []string, known bool, err error)
}

// checkSourcePath is the Source layer's path rule: a valid KILN02 path
// (internal/key) PLUS rules that make listings identical on every operating
// system.  KILN02 stays permissive by design (a backslash or colon is an
// ordinary byte there); the extra strictness lives here, so a repository that
// could not be checked out on Windows is refused everywhere rather than
// producing different file lists, and so different keys, per platform.
//
//	"a\b"      backslash is a separator on Windows
//	"a:b"       colon means drive or alternate data stream on Windows
//	"con.txt"   reserved device name (CON PRN AUX NUL COM1-9 LPT1-9, any
//	            extension, any case) as any path segment
func checkSourcePath(p string) error {
	if err := key.ValidatePath(p); err != nil {
		return err
	}
	if strings.ContainsAny(p, "\\:") {
		return fmt.Errorf("path %q: backslash and colon are not portable", short(p))
	}
	for _, seg := range strings.Split(p, "/") {
		if reservedName(seg) {
			return fmt.Errorf("path %q: reserved Windows device name %q", short(p), short(seg))
		}
	}
	return nil
}

// reservedName reports whether the segment is a Windows device name.
func reservedName(seg string) bool {
	base := seg
	if i := strings.IndexByte(base, '.'); i >= 0 {
		base = base[:i]
	}
	base = strings.ToUpper(strings.TrimRight(base, " "))
	switch base {
	case "CON", "PRN", "AUX", "NUL":
		return true
	}
	if len(base) == 4 && (strings.HasPrefix(base, "COM") || strings.HasPrefix(base, "LPT")) {
		return base[3] >= '1' && base[3] <= '9'
	}
	return false
}

// normRoot validates a root argument and returns it without a trailing form:
// "" for the whole tree, else a valid path.
func normRoot(root string) (string, error) {
	if root == "" || root == "." {
		return "", nil
	}
	if err := checkSourcePath(root); err != nil {
		return "", fmt.Errorf("root: %w", err)
	}
	return root, nil
}

// filter applies the root and pattern rules shared by every implementation,
// validating each path, and returns the sorted result.
func filter(all []string, root string, patterns []string) ([]string, error) {
	root, err := normRoot(root)
	if err != nil {
		return nil, err
	}
	for _, p := range patterns {
		if err := ValidatePattern(p); err != nil {
			return nil, err
		}
	}
	out := []string{}
	for _, p := range all {
		rel := p
		if root != "" {
			if !strings.HasPrefix(p, root+"/") {
				continue
			}
			rel = p[len(root)+1:]
		}
		if !matchAny(patterns, rel) {
			continue
		}
		if err := checkSourcePath(p); err != nil {
			return nil, err
		}
		out = append(out, p)
	}
	sort.Strings(out) // Go string order is bytewise
	return out, nil
}

func sortStrings(s []string) { sort.Strings(s) }

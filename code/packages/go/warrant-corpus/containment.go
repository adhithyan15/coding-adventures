package warrantcorpus

import (
	"os"
	"path/filepath"
	"strings"
)

// # Is this path inside that directory?  (VER04i 9.1, containment profile)
//
// A containment check is a security boundary: the corpus, the binding file and
// the library under test must live inside the repository, or a crafted
// `--corpus` could point the runner at files it was never meant to read.
//
// # The trap: a relative-path string is the wrong thing to inspect
//
// The usual idiom is "compute filepath.Rel(root, child) and see whether it
// climbs out with `..`". That is not enough: across Windows drives Rel errors
// (Node's path.relative returns an absolute target instead), a UNC share has no
// drive, and a directory NAMED `..foo` begins with `..` and is inside. So this
// check never builds a relative string to judge. It is LEXICAL: each path is
// normalised into an anchor (drive, UNC share, or root) plus whole components,
// and the verdict is "same anchor, and every component of root is a leading
// component of child". Whole components also mean `/rr` is not inside `/r`.
//
// The helper takes the path flavour as a parameter so the shared fixture can
// drive the win32 rules from a Linux host. Production passes hostFlavor, the
// same helper with the same rules: there is no variant that skips the device
// rule.
//
//   - win32 compares case-insensitively, accepts `/` and `\` as separators
//     (normalised BEFORE any prefix rule, so `\/?\D:\r` is a device path too),
//     and refuses a device path (`\\?\`, `\\.\`, including `\\?\UNC\...`).
//   - posix is case-sensitive and `\` is an ordinary character; a path with
//     EXACTLY two leading slashes is implementation-defined by POSIX, so like a
//     device path it is never inside anything; three or more collapse to one.
//
// Symlinks are not resolved here; resolveInside does that first.
const (
	flavorPosix = "posix"
	flavorWin32 = "win32"
)

// hostFlavor is the path rules production uses. It is a variable so a test can
// prove the production call path applies the same rules the fixture drives.
var hostFlavor = func() string {
	if filepath.Separator == '\\' {
		return flavorWin32
	}
	return flavorPosix
}()

// lexicalPath normalises text under flavor into an anchor and its components.
// ok == false means a path that is never inside anything (a device path or a
// POSIX `//x` path). The returned comparison key folds case on win32 only.
func lexicalPath(text, flavor string) (anchor string, parts []string, ok bool) {
	var rest string
	switch flavor {
	case flavorPosix:
		if strings.HasPrefix(text, "//") && !strings.HasPrefix(text, "///") {
			return "", nil, false
		}
		if strings.HasPrefix(text, "/") {
			anchor = "/"
		}
		rest = text
	case flavorWin32:
		text = strings.ReplaceAll(text, "/", `\`)
		if strings.HasPrefix(text, `\\?\`) || strings.HasPrefix(text, `\\.\`) {
			return "", nil, false
		}
		anchor, rest = splitWindowsAnchor(text)
	default:
		panic("lexicalPath: unknown flavor " + flavor)
	}
	separator := "/"
	if flavor == flavorWin32 {
		separator = `\`
	}
	for _, part := range strings.Split(rest, separator) {
		switch part {
		case "", ".":
		case "..":
			if len(parts) > 0 {
				parts = parts[:len(parts)-1]
			} else if anchor == "" {
				parts = append(parts, part) // a relative path keeps its leading ".."
			} // else: a `..` at the root stays at the root
		default:
			parts = append(parts, part)
		}
	}
	return anchor, parts, true
}

// splitWindowsAnchor splits a separator-normalised win32 path into its anchor
// (`d:\`, `\\server\share\`, `\` or "") and the remaining components text.
// The anchor is lowercased: win32 comparison is case-insensitive.
func splitWindowsAnchor(text string) (string, string) {
	if len(text) >= 2 && text[1] == ':' && (text[0]|0x20) >= 'a' && (text[0]|0x20) <= 'z' {
		drive := strings.ToLower(text[:2])
		if strings.HasPrefix(text[2:], `\`) {
			return drive + `\`, text[2:]
		}
		return drive, text[2:]
	}
	if strings.HasPrefix(text, `\\`) && !strings.HasPrefix(text, `\\\`) {
		fields := strings.SplitN(text[2:], `\`, 3)
		if len(fields) >= 2 && fields[0] != "" && fields[1] != "" {
			anchor := strings.ToLower(`\\` + fields[0] + `\` + fields[1] + `\`)
			rest := ""
			if len(fields) == 3 {
				rest = fields[2]
			}
			return anchor, rest
		}
	}
	if strings.HasPrefix(text, `\`) {
		return `\`, text
	}
	return "", text
}

// relativeInside returns child relative to root (slash-separated, original
// case) when child lies at or beneath root under flavor's rules, and ok == false
// otherwise. The root itself yields "" with ok == true; callers that need a
// strict descendant reject "" as well.
func relativeInside(root, child, flavor string) (string, bool) {
	rootAnchor, rootParts, rootOK := lexicalPath(root, flavor)
	childAnchor, childParts, childOK := lexicalPath(child, flavor)
	if !rootOK || !childOK || rootAnchor != childAnchor || len(childParts) < len(rootParts) {
		return "", false
	}
	for i, part := range rootParts {
		if flavor == flavorWin32 {
			if !strings.EqualFold(part, childParts[i]) {
				return "", false
			}
		} else if part != childParts[i] {
			return "", false
		}
	}
	return strings.Join(childParts[len(rootParts):], "/"), true
}

// realPath makes path absolute and resolves symlinks, so a link inside the
// repository that points outside it is judged by where it really leads.
func realPath(path string) (string, error) {
	abs, err := filepath.Abs(path)
	if err != nil {
		return "", err
	}
	return filepath.EvalSymlinks(abs)
}

// resolveInside resolves path and root to real paths and returns path relative
// to root, using forward slashes. It refuses (ok == false) a path that does not
// exist, cannot be resolved or lies outside root. With strict, the root itself
// is also refused.
func resolveInside(root, path string, strict bool) (string, bool) {
	realRoot, err := realPath(root)
	if err != nil {
		return "", false
	}
	realChild, err := realPath(path)
	if err != nil {
		return "", false
	}
	rel, ok := relativeInside(realRoot, realChild, hostFlavor)
	if !ok || (strict && rel == "") {
		return "", false
	}
	return filepath.ToSlash(rel), true
}

// requireInside is resolveInside as a refusal: the run does not start.
func requireInside(root, path, what string) string {
	rel, ok := resolveInside(root, path, true)
	if !ok {
		bindingFail("path", "the %s must lie inside the repository root", what)
	}
	return rel
}

// statIsRegular reports whether path names a regular file (not a symlink,
// directory or device); it never follows a final symlink.
func statIsRegular(path string) bool {
	info, err := os.Lstat(path)
	return err == nil && info.Mode().IsRegular()
}

package warrantcorpus

import (
	"os"
	"path/filepath"
	"strings"
)

// # Is this path inside that directory?
//
// A containment check is a security boundary: the corpus, the binding file and
// the library under test must live inside the repository, or a crafted
// `--corpus` could point the runner at files it was never meant to read.
//
// # The trap: Rel does not always give you a relative path
//
// The usual idiom is "compute filepath.Rel(root, child) and see whether it
// climbs out with `..`". That is not enough, in three separate ways.
//
//  1. Across Windows drives there is no way to climb from D:\repo to C:\tmp\x.
//     Go's filepath.Rel returns an *error* there (Node's path.relative returns
//     the absolute target instead). A caller that ignores the error and looks at
//     the empty string it got back would accept it.
//  2. A path module that does return an absolute target (the Node behaviour, or
//     a UNC share) slips past a `..` prefix test, so an absolute result is also
//     "outside".
//  3. The result `..` and the results `..\x` and `../x` are outside, but a
//     directory legitimately named `..foo` is inside and Rel reports it as
//     `..foo`. So the prefix test must be "`..`, or `..` followed by a
//     separator", never a bare strings.HasPrefix(rel, "..").
//
// pathOps carries the five primitives the check needs, so tests can drive a
// Windows-shaped implementation from any host.
type pathOps struct {
	rel   func(base, target string) (string, error)
	isAbs func(path string) bool
	sep   byte
}

// osPath is the host's own path rules.
var osPath = pathOps{
	rel:   filepath.Rel,
	isAbs: filepath.IsAbs,
	sep:   filepath.Separator,
}

// relativeInside returns child relative to root when child lies at or beneath
// root, and ok == false otherwise. The root itself yields "" with ok == true;
// callers that need a strict descendant reject "" as well.
//
// Both arguments should already be absolute and cleaned; resolveInside does that.
func relativeInside(root, child string, ops pathOps) (string, bool) {
	rel, err := ops.rel(root, child)
	if err != nil {
		return "", false // different volume, or no relative form exists
	}
	if ops.isAbs(rel) {
		return "", false
	}
	if rel == ".." || strings.HasPrefix(rel, ".."+string(ops.sep)) {
		return "", false
	}
	if rel == "." {
		rel = "" // the root itself
	}
	return rel, true
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
	rel, ok := relativeInside(realRoot, realChild, osPath)
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

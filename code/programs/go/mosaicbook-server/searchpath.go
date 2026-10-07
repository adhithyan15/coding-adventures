package main

// Extra package search paths (`--package-search-path`).
//
// Every three-file component is compiled with `--package-search-path` set to
// the directory that holds its own package -- the package's siblings. That is
// enough for the component library, where every package lives side by side
// under code/packages/mosaic:
//
//	code/packages/mosaic/
//	    mosaic-pkg-toolkit/     <- a dependency
//	    mosaic-pkg-grid/        <- a dependency
//	    mosaic-pkg-card/        <- the package being previewed; its siblings
//	                               are found
//
// It is not enough for an app. An app's package sits under code/programs/mosaic,
// and its dependencies are back in code/packages/mosaic:
//
//	code/programs/mosaic/
//	    visicalc/               <- the package being previewed
//	code/packages/mosaic/
//	    mosaic-pkg-grid/        <- its dependency: not a sibling, not found
//
// So every app component (VisiCalc, TaskApp, EngramApp, JournalApp) failed to
// compile on every backend: "dependency package `mosaic-pkg-grid` could not be
// found". `--package-search-path` names the extra directories, searched after
// the package's own siblings, so a sibling of the same name still wins:
//
//	mosaicbook-server --root code/programs/mosaic \
//	                  --package-search-path code/packages/mosaic
//
// mosaic-compile splits its own --package-search-path on ':' (on every OS), so
// a directory whose absolute path contains ':' cannot be passed through it at
// all. Such a directory -- a Windows drive path is one -- is refused here with
// a message, rather than handed over as two broken halves.

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// parsePackageSearchPaths turns the flag's value, a list in the OS's own list
// syntax (':' on Unix, ';' on Windows), into absolute directories, in order,
// without repeats. Empty entries are skipped. Every entry must be an existing
// directory, and none may contain ':' (see above).
func parsePackageSearchPaths(list string) ([]string, error) {
	var paths []string
	for _, entry := range filepath.SplitList(list) {
		if entry == "" {
			continue
		}
		abs, err := packageSearchDir(entry)
		if err != nil {
			return nil, err
		}
		if !containsString(paths, abs) {
			paths = append(paths, abs)
		}
	}
	return paths, nil
}

// packageSearchDir is one entry of the list, made absolute and checked: an
// existing directory whose absolute path holds no ':'. On Unix the list
// itself is split on ':', so the colon check matters on Windows, where a
// drive path always has one.
func packageSearchDir(entry string) (string, error) {
	abs, err := filepath.Abs(entry)
	if err != nil {
		return "", fmt.Errorf("package search path %q: %v", entry, err)
	}
	if strings.Contains(abs, ":") {
		return "", fmt.Errorf(
			"package search path %q contains ':', which mosaic-compile reads as its list separator",
			entry,
		)
	}
	info, err := os.Stat(abs)
	if err != nil || !info.IsDir() {
		return "", fmt.Errorf("package search path %q is not a directory", entry)
	}
	return abs, nil
}

// withPackageSearchPaths appends extras after base, without repeats, joined
// with ':' as mosaic-compile expects:
//
//	base        extras          result
//	""          []              ""
//	"/a"        []              "/a"
//	"/a"        ["/b", "/a"]    "/a:/b"
//	""          ["/b"]          "/b"
func withPackageSearchPaths(base string, extras []string) string {
	var joined []string
	if base != "" {
		joined = append(joined, base)
	}
	for _, extra := range extras {
		if !containsString(joined, extra) {
			joined = append(joined, extra)
		}
	}
	return strings.Join(joined, ":")
}

func containsString(values []string, value string) bool {
	for _, existing := range values {
		if existing == value {
			return true
		}
	}
	return false
}

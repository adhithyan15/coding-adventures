// Package hasher computes SHA256 hashes for package source files.
//
// # Why hashing?
//
// The core of incremental builds is change detection. If nothing changed
// in a package's source files, there is no reason to rebuild it. We detect
// changes by computing a SHA256 hash of all relevant source files and
// comparing it against the cached hash from the last build.
//
// # How hashing works
//
// The hashing algorithm is deterministic — given the same files with the
// same contents, it always produces the same hash. Here is the procedure:
//
//  1. Collect all source files in the package directory, filtered by the
//     language's relevant extensions. Always include BUILD files.
//  2. Sort the file list lexicographically by relative path. This ensures
//     that file ordering does not affect the hash.
//  3. Frame each repository-relative UTF-8 path and exact raw content with
//     unsigned 64-bit big-endian byte lengths.
//  4. SHA256-hash that single unambiguous stream.
//
// This framed hashing means:
//   - Reordering files doesn't change the hash (we sort first).
//   - Adding or removing a file changes the hash.
//   - Modifying any file's contents changes the hash.
//
// # Dependency hashing
//
// A package should be rebuilt if any of its transitive dependencies changed.
// HashDeps takes a package's dependency information and produces a single
// hash representing the state of all its dependencies.
package hasher

import (
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"unicode/utf8"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// collectSourceFiles walks the package directory and returns all source
// files relevant to the package's language. Files are sorted by their
// relative path for deterministic hashing.
//
// The collection rules:
//   - BUILD, BUILD_mac, BUILD_linux, and BUILD_windows are always included.
//   - Files matching the language's extensions are included.
//   - Special filenames (go.mod, Gemfile, etc.) are included.
//   - Everything else is ignored.
func collectSourceFiles(pkg discovery.Package) []string {
	files, _ := collectSourceFilesChecked(pkg)
	return files
}

func collectSourceFilesChecked(pkg discovery.Package) ([]string, error) {
	return collectSourceInputsChecked(pkg, false)
}

// resolveDeclaredSrcs converts the declared source patterns from a Starlark
// BUILD file into actual file paths. Each pattern is resolved relative to
// the package directory. Glob patterns (like "src/**/*.py") are expanded.
// The BUILD file itself is always included.
//
// Files are sorted by relative path for deterministic hashing.
func resolveDeclaredSrcs(pkg discovery.Package) []string {
	files, _ := resolveDeclaredSrcsChecked(pkg)
	return files
}

func resolveDeclaredSrcsChecked(pkg discovery.Package) ([]string, error) {
	return collectSourceInputsChecked(pkg, true)
}

func collectSourceInputsChecked(pkg discovery.Package, declaredMode bool) ([]string, error) {
	sourceLanguage, packageRoot, err := sourceInputProfile(pkg)
	if err != nil {
		return nil, fmt.Errorf("repository package identity is invalid")
	}
	language, ok := languageSourceInputRegistryByName[sourceLanguage]
	if !ok {
		return nil, fmt.Errorf("unsupported source language")
	}
	if packageRoot != "" {
		parts := strings.Split(packageRoot, "/")
		languageRoot := len(parts) >= 4 && parts[0] == "code" &&
			(parts[1] == "packages" || parts[1] == "programs") && parts[2] == sourceLanguage
		siteRoot := len(parts) == 3 && parts[0] == "code" && parts[1] == "sites" && sourceLanguage == "typescript" &&
			matchesPackageExactRoot(language.PackageExactInputs, packageRoot)
		if !languageRoot && !siteRoot {
			return nil, fmt.Errorf("repository package identity is invalid")
		}
	}

	files := make([]string, 0)
	err = walkSourceFiles(pkg.Path, func(path string, entry os.DirEntry) error {
		relativePath, err := portableRelativePath(pkg.Path, path)
		if err != nil {
			return err
		}
		if matchesSourceInput(
			language,
			packageRoot,
			relativePath,
			entry.Name(),
			!strings.Contains(relativePath, "/"),
			declaredMode,
			pkg.DeclaredSrcs,
		) {
			files = append(files, path)
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	if err := sortPortablePaths(files, pkg.Path); err != nil {
		return nil, err
	}
	return files, nil
}

// sourceInputProfile maps repository-owned site packages to the TypeScript
// source-input contract without changing their legacy unknown/* graph identity.
// Arbitrary unknown packages remain unsupported and fail before traversal.
func sourceInputProfile(pkg discovery.Package) (string, string, error) {
	if pkg.Language == "unknown" {
		packageRoot, found, err := repositoryRelativeSiteRoot(pkg.Path)
		if err != nil {
			return "", "", err
		}
		if found {
			typescript := languageSourceInputRegistryByName["typescript"]
			if !matchesPackageExactRoot(typescript.PackageExactInputs, packageRoot) {
				return "", "", fmt.Errorf("site package is not registered")
			}
			return "typescript", packageRoot, nil
		}
	}
	packageRoot, err := exactPackageSourceRoot(pkg)
	return pkg.Language, packageRoot, err
}

// repositoryRelativeSiteRoot grants the legacy site bridge only to an exact
// code/sites/<name> path below the nearest real Git repository root. A matching
// filesystem suffix, including one outside or nested inside a repository, has
// no source-selector authority.
func repositoryRelativeSiteRoot(packagePath string) (string, bool, error) {
	absolute, err := filepath.Abs(packagePath)
	if err != nil {
		return "", false, fmt.Errorf("cannot resolve site package path")
	}
	repositoryRoot, found, err := findRepositoryRoot(absolute)
	if err != nil || !found {
		return "", false, err
	}
	canonicalPackage, err := filepath.EvalSymlinks(absolute)
	if err != nil {
		return "", false, fmt.Errorf("cannot resolve site package path")
	}
	canonicalRepository, err := filepath.EvalSymlinks(repositoryRoot)
	if err != nil {
		return "", false, fmt.Errorf("cannot resolve repository root")
	}
	canonicalRelative, err := filepath.Rel(canonicalRepository, canonicalPackage)
	if err != nil || canonicalRelative == ".." || strings.HasPrefix(canonicalRelative, ".."+string(filepath.Separator)) {
		return "", false, fmt.Errorf("site package is not relative to repository root")
	}
	relative, err := filepath.Rel(repositoryRoot, absolute)
	if err != nil {
		return "", false, fmt.Errorf("site package is not relative to repository root")
	}
	candidate := filepath.ToSlash(relative)
	if err := validateRepositoryPath(candidate); err != nil {
		return "", false, err
	}
	parts := strings.Split(candidate, "/")
	if len(parts) != 3 || parts[0] != "code" || parts[1] != "sites" {
		return "", false, nil
	}
	return candidate, true, nil
}

func findRepositoryRoot(start string) (string, bool, error) {
	current := filepath.Clean(start)
	for {
		gitPath := filepath.Join(current, ".git")
		info, err := os.Lstat(gitPath)
		if err == nil {
			if info.Mode()&os.ModeSymlink != 0 || (!info.IsDir() && !info.Mode().IsRegular()) {
				return "", false, fmt.Errorf("repository marker is not stable")
			}
			return current, true, nil
		}
		if !os.IsNotExist(err) {
			return "", false, fmt.Errorf("cannot inspect repository marker")
		}
		parent := filepath.Dir(current)
		if parent == current {
			return "", false, nil
		}
		current = parent
	}
}

func matchesPackageExactRoot(rules []packageExactSourceInput, packageRoot string) bool {
	for _, rule := range rules {
		if rule.PackageRoot == packageRoot {
			return true
		}
	}
	return false
}

// walkSourceFiles enumerates only regular lexical descendants of root. It
// prunes the exact shared generated-directory registry before matching either
// extension or declared-source rules and never traverses link/reparse entries.
func walkSourceFiles(root string, visit func(string, os.DirEntry) error) error {
	cleanRoot := filepath.Clean(root)
	rootInfo, err := os.Lstat(cleanRoot)
	if err != nil || !rootInfo.IsDir() || rootInfo.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(rootInfo) {
		return fmt.Errorf("package root is not a stable directory")
	}
	return filepath.WalkDir(cleanRoot, func(path string, entry os.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if path == cleanRoot {
			return nil
		}
		info, err := entry.Info()
		if err != nil {
			return err
		}
		linked := info.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(info)
		if entry.IsDir() {
			if linked || containsString(languageSourceInputRegistry.UniversalInputs.GeneratedDirectoryComponents, entry.Name()) {
				return filepath.SkipDir
			}
			return nil
		}
		if linked || !info.Mode().IsRegular() {
			return nil
		}
		return visit(path, entry)
	})
}

// hasWindowsReparsePoint avoids a platform-specific source file while still
// rejecting NTFS junctions and other reparse entries. On Windows the Sys value
// exposes a FileAttributes field; FILE_ATTRIBUTE_REPARSE_POINT is 0x400.
func hasWindowsReparsePoint(info os.FileInfo) bool {
	system := reflect.ValueOf(info.Sys())
	if !system.IsValid() {
		return false
	}
	if system.Kind() == reflect.Pointer {
		if system.IsNil() {
			return false
		}
		system = system.Elem()
	}
	if system.Kind() != reflect.Struct {
		return false
	}
	attributes := system.FieldByName("FileAttributes")
	return attributes.IsValid() && attributes.CanUint() && attributes.Uint()&0x400 != 0
}

func portableRelativePath(root, path string) (string, error) {
	relative, err := filepath.Rel(filepath.Clean(root), filepath.Clean(path))
	if err != nil {
		return "", fmt.Errorf("source path is not relative to package root")
	}
	portable := filepath.ToSlash(relative)
	if err := validateRepositoryPath(portable); err != nil {
		return "", fmt.Errorf("source path is not portable UTF-8")
	}
	return portable, nil
}

func sortPortablePaths(files []string, root string) error {
	relatives := make(map[string]string, len(files))
	for _, file := range files {
		relative, err := portableRelativePath(root, file)
		if err != nil {
			return err
		}
		relatives[file] = relative
	}
	sort.Slice(files, func(i, j int) bool {
		return relatives[files[i]] < relatives[files[j]]
	})
	return nil
}

func repositoryRelativePackagePath(pkg discovery.Package) (string, error) {
	if candidate, found, err := canonicalRepositoryPackagePath(pkg.Path); err != nil || found {
		return candidate, err
	}

	identity := strings.Split(pkg.Name, "/")
	var candidate string
	if len(identity) == 3 && identity[1] == "programs" {
		candidate = "code/programs/" + identity[0] + "/" + identity[2]
	} else if len(identity) == 2 {
		candidate = "code/packages/" + identity[0] + "/" + identity[1]
	} else {
		return "", fmt.Errorf("cannot derive repository-relative package path")
	}
	if err := validateRepositoryPath(candidate); err != nil {
		return "", err
	}
	return candidate, nil
}

func exactPackageSourceRoot(pkg discovery.Package) (string, error) {
	candidate, found, err := canonicalRepositoryPackagePath(pkg.Path)
	if err != nil || !found {
		return candidate, err
	}
	parts := strings.Split(candidate, "/")
	if len(parts) < 4 || (parts[1] != "packages" && parts[1] != "programs") || parts[2] != pkg.Language {
		return "", fmt.Errorf("canonical package path does not match language")
	}
	return candidate, nil
}

func canonicalRepositoryPackagePath(packagePath string) (string, bool, error) {
	normalized := filepath.ToSlash(filepath.Clean(packagePath))
	parts := strings.Split(normalized, "/")
	canonicalStart := -1
	for index := 0; index+1 < len(parts); index++ {
		if parts[index] == "code" && (parts[index+1] == "packages" || parts[index+1] == "programs" || parts[index+1] == "sites") {
			canonicalStart = index
		}
	}
	if canonicalStart >= 0 {
		candidate := strings.Join(parts[canonicalStart:], "/")
		if err := validateRepositoryPath(candidate); err != nil {
			return "", false, err
		}
		return candidate, true, nil
	}
	return "", false, nil
}

func validateRepositoryPath(path string) error {
	if path == "" || !utf8.ValidString(path) || strings.ContainsRune(path, '\x00') || strings.Contains(path, "\\") || strings.HasPrefix(path, "/") {
		return fmt.Errorf("path is not portable UTF-8")
	}
	for _, component := range strings.Split(path, "/") {
		if component == "" || component == "." || component == ".." {
			return fmt.Errorf("path is not portable UTF-8")
		}
	}
	return nil
}

func ensureUnlinkedComponents(root, path string) error {
	relative, err := filepath.Rel(filepath.Clean(root), filepath.Clean(path))
	if err != nil || relative == ".." || strings.HasPrefix(relative, ".."+string(filepath.Separator)) {
		return fmt.Errorf("source path escapes package root")
	}
	current := filepath.Clean(root)
	components := []string{"."}
	if relative != "." {
		components = append(components, strings.Split(relative, string(filepath.Separator))...)
	}
	for _, component := range components {
		if component != "." {
			current = filepath.Join(current, component)
		}
		info, err := os.Lstat(current)
		if err != nil {
			return fmt.Errorf("source path is unavailable")
		}
		if info.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(info) {
			return fmt.Errorf("source link component is not hashable")
		}
	}
	return nil
}

func writeFileFrame(hash io.Writer, packageRoot, root, path string) error {
	relative, err := portableRelativePath(root, path)
	if err != nil {
		return err
	}
	repositoryPath := packageRoot + "/" + relative
	if err := validateRepositoryPath(repositoryPath); err != nil {
		return err
	}
	pathBytes := []byte(repositoryPath)
	if err := binary.Write(hash, binary.BigEndian, uint64(len(pathBytes))); err != nil {
		return err
	}
	if _, err := hash.Write(pathBytes); err != nil {
		return err
	}
	return writeStableContentFrame(hash, root, path)
}

func writeStableContentFrame(hash io.Writer, root, path string) error {
	return writeStableContentFrameBounded(hash, root, path, nil, 0, nil, 0)
}

func writeStableContentFrameBounded(
	hash io.Writer,
	root, path string,
	expected os.FileInfo,
	maxFileBytes int64,
	totalBytesRead *int64,
	totalByteLimit int64,
) error {
	return writeStableContentFrameBoundedWithHook(hash, root, path, expected, maxFileBytes, totalBytesRead, totalByteLimit, nil)
}

// The hook is an internal test seam used to force path replacement or file
// growth after opening the descriptor and before streaming its bytes.
func writeStableContentFrameBoundedWithHook(
	hash io.Writer,
	root, path string,
	expected os.FileInfo,
	maxFileBytes int64,
	totalBytesRead *int64,
	totalByteLimit int64,
	afterOpen func() error,
) error {
	if err := ensureUnlinkedComponents(root, path); err != nil {
		return err
	}
	before, err := os.Lstat(path)
	if err != nil || !before.Mode().IsRegular() || before.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(before) ||
		(expected != nil && !sameFileSnapshot(expected, before)) {
		return fmt.Errorf("source is not a stable regular file")
	}
	source, err := os.Open(path)
	if err != nil {
		return fmt.Errorf("source is not readable")
	}
	defer source.Close()
	opened, err := source.Stat()
	if err != nil || !opened.Mode().IsRegular() || !sameFileSnapshot(before, opened) ||
		(expected != nil && !sameFileSnapshot(expected, opened)) {
		return fmt.Errorf("source changed before hashing")
	}
	if afterOpen != nil {
		if err := afterOpen(); err != nil {
			return err
		}
	}
	contentLength := opened.Size()
	if contentLength < 0 || (maxFileBytes > 0 && contentLength > maxFileBytes) {
		return fmt.Errorf("source length is invalid")
	}
	allowedBytes := int64(0)
	limited := maxFileBytes > 0
	if maxFileBytes > 0 {
		allowedBytes = maxFileBytes
	}
	if totalBytesRead != nil {
		remaining := totalByteLimit - *totalBytesRead
		if remaining < 0 {
			return fmt.Errorf("repository tree exceeds its byte limit")
		}
		if !limited || remaining < allowedBytes {
			allowedBytes = remaining
		}
		limited = true
	}
	if limited && contentLength > allowedBytes {
		return fmt.Errorf("repository tree exceeds its byte limit")
	}
	if err := binary.Write(hash, binary.BigEndian, uint64(contentLength)); err != nil {
		return err
	}
	var reader io.Reader = source
	if limited {
		reader = io.LimitReader(source, allowedBytes+1)
	}
	written, err := io.CopyBuffer(hash, reader, make([]byte, 8192))
	if err != nil {
		return fmt.Errorf("source read failed")
	}
	after, err := source.Stat()
	pathAfter, pathErr := os.Lstat(path)
	if err != nil || pathErr != nil || written != contentLength ||
		!sameFileSnapshot(opened, after) || !sameFileSnapshot(opened, pathAfter) ||
		(expected != nil && !sameFileSnapshot(expected, pathAfter)) {
		return fmt.Errorf("source changed while hashing")
	}
	if totalBytesRead != nil {
		*totalBytesRead += written
		if *totalBytesRead > totalByteLimit {
			return fmt.Errorf("repository tree exceeds its byte limit")
		}
	}
	if err := ensureUnlinkedComponents(root, path); err != nil {
		return err
	}
	return nil
}

func sameFileSnapshot(expected, actual os.FileInfo) bool {
	return expected != nil && actual != nil && os.SameFile(expected, actual) &&
		expected.Mode() == actual.Mode() && expected.Size() == actual.Size() &&
		expected.ModTime().Equal(actual.ModTime())
}

// HashPackage computes a SHA256 hash representing all source files in
// the package. The hash changes if any source file is added, removed,
// or modified.
//
// When a package has DeclaredSrcs (from a Starlark BUILD file), we hash
// ONLY those declared files — this is strict mode. When DeclaredSrcs is
// empty (shell BUILD files), we fall back to extension-based collection.
//
// If the package has no source files, we hash the empty string for
// consistency — every package gets a hash, even empty ones.
func HashPackage(pkg discovery.Package) (string, error) {
	return HashPackageWithRepositoryInputs(pkg, "", nil)
}

// HashPackageWithRepositoryInputs hashes exact repository-relative regular
// files. It never infers recursive tree inputs from the current file kind.
func HashPackageWithRepositoryInputs(pkg discovery.Package, repoRoot string, repositoryInputs []string) (string, error) {
	return HashPackageWithRepositoryInputsAndTrees(pkg, repoRoot, repositoryInputs, nil)
}

// HashPackageWithRepositoryInputsAndTrees adds separately registered file and
// tree inputs. A path's kind comes from the caller's trusted registration, not
// from a filesystem check that silently changes the registration's meaning.
func HashPackageWithRepositoryInputsAndTrees(
	pkg discovery.Package,
	repoRoot string,
	repositoryFiles []string,
	repositoryTrees []string,
) (string, error) {
	var (
		files []string
		err   error
	)
	if len(pkg.DeclaredSrcs) > 0 {
		// Strict mode: hash only declared sources.
		files, err = resolveDeclaredSrcsChecked(pkg)
	} else {
		// Legacy mode: extension-based collection.
		files, err = collectSourceFilesChecked(pkg)
	}
	if err != nil {
		return "", fmt.Errorf("source collection failed")
	}
	packageRoot, err := repositoryRelativePackagePath(pkg)
	if err != nil {
		return "", fmt.Errorf("repository identity is invalid")
	}
	hash := sha256.New()
	for _, file := range files {
		if err := writeFileFrame(hash, packageRoot, pkg.Path, file); err != nil {
			return "", fmt.Errorf("source input is not stable")
		}
	}
	if len(repositoryFiles)+len(repositoryTrees) > 0 {
		if repoRoot == "" {
			return "", fmt.Errorf("repository input root is required")
		}
		seen := make(map[string]struct{}, len(repositoryFiles)+len(repositoryTrees))
		for _, input := range repositoryFiles {
			if err := validateRepositoryInputPath(input, seen); err != nil {
				return "", err
			}
			repositoryPath := filepath.Join(repoRoot, filepath.FromSlash(input))
			if err := ensureUnlinkedComponents(repoRoot, repositoryPath); err != nil {
				return "", fmt.Errorf("repository file input is not stable")
			}
			if err := writeFileFrame(hash, packageRoot, repoRoot, repositoryPath); err != nil {
				return "", fmt.Errorf("repository file input is not a stable regular file")
			}
		}
		for _, input := range repositoryTrees {
			if err := validateRepositoryInputPath(input, seen); err != nil {
				return "", err
			}
		}
		if err := hashRepositoryTreeInputs(hash, packageRoot, repoRoot, repositoryTrees, defaultRepositoryTreeLimits); err != nil {
			return "", err
		}
	}
	return hex.EncodeToString(hash.Sum(nil)), nil
}

// hashRepositoryTreeInputs applies the byte budget independently to each
// registered tree and compares its bounded before/after inventory before moving
// to the next tree.
func hashRepositoryTreeInputs(hash io.Writer, packageRoot, repoRoot string, trees []string, limits repositoryTreeLimits) error {
	for _, input := range trees {
		snapshot, err := collectRepositoryTreeSnapshot(repoRoot, input, limits)
		if err != nil {
			return fmt.Errorf("repository tree input %q is not stable: %w", input, err)
		}
		var treeBytesRead int64
		if err := writeRepositoryTreeFrame(hash, packageRoot, repoRoot, input, snapshot, &treeBytesRead, limits); err != nil {
			return fmt.Errorf("repository tree input %q is not stable: %w", input, err)
		}
		after, err := collectRepositoryTreeSnapshot(repoRoot, input, limits)
		if err != nil || !sameRepositoryTreeSnapshot(snapshot, after) {
			return fmt.Errorf("repository tree changed while hashing")
		}
	}
	return nil
}

const (
	maxRepositoryTreeDepth     = 32
	maxRepositoryTreeEntries   = 100_000
	maxRepositoryTreeFileBytes = 8 * 1024 * 1024
	maxRepositoryTreeTotal     = 128 * 1024 * 1024
	maxRepositoryTreePathBytes = 4096
	maxRepositoryTreePathTotal = 32 * 1024 * 1024
	repositoryTreeReadPage     = 256
)

type repositoryTreeLimits struct {
	maxDepth, maxEntries, maxPathBytes, maxPathTotal int
	maxFileBytes, maxTotalBytes                      int64
}

var defaultRepositoryTreeLimits = repositoryTreeLimits{
	maxDepth: maxRepositoryTreeDepth, maxEntries: maxRepositoryTreeEntries,
	maxPathBytes: maxRepositoryTreePathBytes, maxPathTotal: maxRepositoryTreePathTotal,
	maxFileBytes: maxRepositoryTreeFileBytes, maxTotalBytes: maxRepositoryTreeTotal,
}

type repositoryTreeEntry struct {
	relativePath string
	absolutePath string
	directory    bool
	info         os.FileInfo
}

type repositoryTreeSnapshot struct {
	rootInfo os.FileInfo
	entries  []repositoryTreeEntry
}

type repositoryTreeInventoryBudget struct {
	entries, pathBytes int
}

func validateRepositoryInputPath(input string, seen map[string]struct{}) error {
	if filepath.ToSlash(filepath.Clean(filepath.FromSlash(input))) != input || filepath.IsAbs(input) {
		return fmt.Errorf("repository input path is invalid")
	}
	if err := validateRepositoryPath(input); err != nil {
		return fmt.Errorf("repository input path is invalid")
	}
	if _, exists := seen[input]; exists {
		return fmt.Errorf("repository input kind is ambiguous")
	}
	seen[input] = struct{}{}
	return nil
}

// collectRepositoryTreeSnapshot uses bounded paged directory reads. It counts
// each child before retaining it and records directory and file identities so
// a final inventory can detect replacement, addition, removal, or mutation.
func collectRepositoryTreeSnapshot(repoRoot, input string, limits repositoryTreeLimits) (repositoryTreeSnapshot, error) {
	root := filepath.Join(repoRoot, filepath.FromSlash(input))
	if err := ensureUnlinkedComponents(repoRoot, root); err != nil {
		return repositoryTreeSnapshot{}, err
	}
	rootInfo, err := os.Lstat(root)
	if err != nil || !rootInfo.IsDir() || rootInfo.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(rootInfo) {
		return repositoryTreeSnapshot{}, fmt.Errorf("registered tree is not a directory")
	}
	if limits.maxEntries < 1 || limits.maxDepth < 0 || limits.maxPathBytes < 1 || limits.maxPathTotal < 1 {
		return repositoryTreeSnapshot{}, fmt.Errorf("repository tree limits are invalid")
	}
	budget := repositoryTreeInventoryBudget{entries: 1, pathBytes: len(input)}
	if budget.pathBytes > limits.maxPathBytes || budget.pathBytes > limits.maxPathTotal {
		return repositoryTreeSnapshot{}, fmt.Errorf("repository tree path exceeds its limit")
	}
	snapshot := repositoryTreeSnapshot{rootInfo: rootInfo}
	if err := inventoryRepositoryDirectory(repoRoot, root, input, 0, limits, &budget, &snapshot); err != nil {
		return repositoryTreeSnapshot{}, err
	}
	sort.Slice(snapshot.entries, func(i, j int) bool {
		return snapshot.entries[i].relativePath < snapshot.entries[j].relativePath
	})
	return snapshot, nil
}

func inventoryRepositoryDirectory(
	repoRoot, dirPath, dirRelative string,
	depth int,
	limits repositoryTreeLimits,
	budget *repositoryTreeInventoryBudget,
	snapshot *repositoryTreeSnapshot,
) error {
	if depth > limits.maxDepth {
		return fmt.Errorf("repository tree is nested too deeply")
	}
	if err := ensureUnlinkedComponents(repoRoot, dirPath); err != nil {
		return err
	}
	before, err := os.Lstat(dirPath)
	if err != nil || !before.IsDir() || before.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(before) {
		return fmt.Errorf("repository tree directory is not stable")
	}
	dir, err := os.Open(dirPath)
	if err != nil {
		return err
	}
	opened, err := dir.Stat()
	if err != nil || !sameFileSnapshot(before, opened) {
		_ = dir.Close()
		return fmt.Errorf("repository tree directory changed before reading")
	}

	children := make([]repositoryTreeEntry, 0)
	for {
		remaining := limits.maxEntries - budget.entries
		page := repositoryTreeReadPage
		if remaining < page {
			page = remaining
		}
		if page < 1 {
			page = 1 // one unretained sentinel detects an entry beyond the limit
		}
		batch, readErr := dir.ReadDir(page)
		for _, entry := range batch {
			if budget.entries >= limits.maxEntries {
				_ = dir.Close()
				return fmt.Errorf("repository tree has too many entries")
			}
			budget.entries++
			name := entry.Name()
			relative := name
			if dirRelative != "" {
				relative = dirRelative + "/" + name
			}
			absolute := filepath.Join(dirPath, name)
			canonicalPath, err := portableRelativePath(repoRoot, absolute)
			if err != nil {
				_ = dir.Close()
				return err
			}
			if err := validateRepositoryPath(canonicalPath); err != nil {
				_ = dir.Close()
				return err
			}
			pathBytes := len(canonicalPath)
			if pathBytes > limits.maxPathBytes || budget.pathBytes > limits.maxPathTotal-pathBytes {
				_ = dir.Close()
				return fmt.Errorf("repository tree paths exceed their limit")
			}
			budget.pathBytes += pathBytes
			if err := ensureUnlinkedComponents(repoRoot, absolute); err != nil {
				_ = dir.Close()
				return err
			}
			info, err := os.Lstat(absolute)
			if err != nil || info.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(info) {
				_ = dir.Close()
				return fmt.Errorf("repository tree contains an unstable or linked entry")
			}
			child := repositoryTreeEntry{relativePath: relative, absolutePath: absolute, info: info}
			switch {
			case info.IsDir():
				child.directory = true
			case info.Mode().IsRegular():
				if info.Size() < 0 || info.Size() > limits.maxFileBytes {
					_ = dir.Close()
					return fmt.Errorf("repository tree file exceeds its limit")
				}
			default:
				_ = dir.Close()
				return fmt.Errorf("repository tree contains a special file")
			}
			children = append(children, child)
		}
		if readErr != nil {
			if readErr == io.EOF {
				break
			}
			_ = dir.Close()
			return readErr
		}
		if len(batch) == 0 {
			break
		}
	}
	fdAfter, fdErr := dir.Stat()
	closeErr := dir.Close()
	pathAfter, pathErr := os.Lstat(dirPath)
	if fdErr != nil || closeErr != nil || pathErr != nil ||
		!sameFileSnapshot(before, fdAfter) || !sameFileSnapshot(before, pathAfter) {
		return fmt.Errorf("repository tree directory changed while reading")
	}
	if err := ensureUnlinkedComponents(repoRoot, dirPath); err != nil {
		return err
	}
	sort.Slice(children, func(i, j int) bool {
		return children[i].relativePath < children[j].relativePath
	})
	for _, child := range children {
		snapshot.entries = append(snapshot.entries, child)
		if child.directory {
			if err := inventoryRepositoryDirectory(repoRoot, child.absolutePath, child.relativePath, depth+1, limits, budget, snapshot); err != nil {
				return err
			}
		}
	}
	return nil
}

func sameRepositoryTreeSnapshot(before, after repositoryTreeSnapshot) bool {
	if !sameFileSnapshot(before.rootInfo, after.rootInfo) || len(before.entries) != len(after.entries) {
		return false
	}
	for i := range before.entries {
		left, right := before.entries[i], after.entries[i]
		if left.relativePath != right.relativePath || left.directory != right.directory ||
			!sameFileSnapshot(left.info, right.info) {
			return false
		}
	}
	return true
}

// writeRepositoryTreeFrame uses a separate domain and an explicit file count,
// so a tree cannot collide with a sequence of ordinary repository file inputs.
func writeRepositoryTreeFrame(hash io.Writer, packageRoot, repoRoot, input string, snapshot repositoryTreeSnapshot, totalBytesRead *int64, limits repositoryTreeLimits) error {
	if _, err := io.WriteString(hash, "coding-adventures/build-tool-repository-tree/v1\x00"); err != nil {
		return err
	}
	treePath := packageRoot + "/" + input
	if err := writeLengthPrefixed(hash, []byte(treePath)); err != nil {
		return err
	}
	fileCount := 0
	for _, entry := range snapshot.entries {
		if !entry.directory {
			fileCount++
		}
	}
	if err := binary.Write(hash, binary.BigEndian, uint64(fileCount)); err != nil {
		return err
	}
	for _, entry := range snapshot.entries {
		if entry.directory {
			continue
		}
		relative, err := portableRelativePath(repoRoot, entry.absolutePath)
		if err != nil {
			return err
		}
		repositoryPath := packageRoot + "/" + relative
		if err := validateRepositoryPath(repositoryPath); err != nil {
			return err
		}
		if err := writeLengthPrefixed(hash, []byte(repositoryPath)); err != nil {
			return err
		}
		if _, err := hash.Write([]byte{1}); err != nil {
			return err
		}
		if err := writeStableContentFrameBounded(hash, repoRoot, entry.absolutePath, entry.info, limits.maxFileBytes, totalBytesRead, limits.maxTotalBytes); err != nil {
			return err
		}
	}
	return nil
}

func writeLengthPrefixed(w io.Writer, data []byte) error {
	if err := binary.Write(w, binary.BigEndian, uint64(len(data))); err != nil {
		return err
	}
	_, err := w.Write(data)
	return err
}

// HashDeps computes a SHA256 hash of all transitive dependency hashes.
//
// If any transitive dependency's source files changed, this hash will
// change too, triggering a rebuild of the dependent package. This is
// how we propagate changes through the dependency tree.
//
// In our graph, edges go dep → pkg (dependency points to dependent).
// So a package's dependencies are found by following reverse edges
// (predecessors). We use TransitiveDependents which follows forward
// edges from a given node — but here we need the reverse: the packages
// that this package depends ON.
//
// Wait — actually, in our graph convention:
//   - Edge A → B means "A must be built before B" (B depends on A)
//   - So B's dependencies are its predecessors
//   - TransitiveDependents(B) gives everything that depends on B (forward)
//   - We need "transitive dependencies of B" = everything B depends on
//
// We collect predecessors transitively by walking reverse edges.
func HashDeps(
	packageName string,
	graph *directedgraph.Graph,
	packageHashes map[string]string,
) (string, error) {
	if !graph.HasNode(packageName) {
		h := sha256.Sum256([]byte(""))
		return hex.EncodeToString(h[:]), nil
	}

	// Collect all transitive dependencies (packages this one depends on).
	// In our graph, edge A→B means B depends on A. So B's deps are its
	// predecessors. We walk backwards (reverse edges) from packageName.
	transitiveDeps := collectTransitivePredecessors(packageName, graph)

	if len(transitiveDeps) == 0 {
		h := sha256.Sum256([]byte(""))
		return hex.EncodeToString(h[:]), nil
	}

	// Sort for determinism, concatenate hashes, hash again.
	sorted := make([]string, 0, len(transitiveDeps))
	for dep := range transitiveDeps {
		if hash, ok := packageHashes[dep]; !ok || hash == "" {
			return "", fmt.Errorf("dependency package hash is unavailable")
		}
		sorted = append(sorted, dep)
	}
	sort.Strings(sorted)

	var combined strings.Builder
	for _, dep := range sorted {
		combined.WriteString(packageHashes[dep])
	}

	h := sha256.Sum256([]byte(combined.String()))
	return hex.EncodeToString(h[:]), nil
}

// collectTransitivePredecessors walks backwards through the graph from
// the given node, collecting all nodes it transitively depends on.
//
// In our graph, edge A→B means "B depends on A". So to find everything
// that packageName depends on, we follow predecessors (reverse edges).
func collectTransitivePredecessors(node string, graph *directedgraph.Graph) map[string]bool {
	visited := make(map[string]bool)

	// Get direct predecessors to start.
	preds, err := graph.Predecessors(node)
	if err != nil {
		return visited
	}

	// BFS through predecessors.
	queue := make([]string, len(preds))
	copy(queue, preds)
	for _, p := range preds {
		visited[p] = true
	}

	for len(queue) > 0 {
		current := queue[0]
		queue = queue[1:]

		morePreds, err := graph.Predecessors(current)
		if err != nil {
			continue
		}
		for _, pred := range morePreds {
			if !visited[pred] {
				visited[pred] = true
				queue = append(queue, pred)
			}
		}
	}

	return visited
}

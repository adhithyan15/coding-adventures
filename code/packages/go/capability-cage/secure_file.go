// Secure filesystem wrappers.
//
// These functions are drop-in replacements for the corresponding os.* stdlib
// functions, with a capability check injected before delegation.
//
// Usage pattern:
//
//	// Before (raw OS call — flagged by cap/no-raw-io linter rule):
//	data, err := os.ReadFile(grammarPath)
//
//	// After (capability-checked):
//	data, err := cage.ReadFile(Manifest, grammarPath)
//
// If the manifest does not declare fs:read for the requested path, ReadFile
// returns a CapabilityViolationError immediately without touching the OS.
// If the check passes, ReadFile delegates to the package-level Backend
// (OpenBackend by default, which calls os.ReadFile).
//
// The Manifest parameter is always first, making the capability declaration
// visible at each call site. This is intentional: unlike Java's checked
// exceptions, Go has no way to enforce this in the type system, so making
// it visible in the signature is the next best thing.
package capabilitycage

import "os"

// ReadFile checks fs:read:{path} against m, then reads the named file.
//
// The path is used for BOTH the capability check and the OS read. This works
// when the path in the manifest matches the path passed at runtime (e.g. when
// both are relative paths). For packages that compute an absolute runtime path
// using runtime.Caller (e.g. to find a grammar file next to the source), use
// ReadFileAt instead, which separates the declared logical path from the
// actual OS path.
func ReadFile(m *Manifest, path string) ([]byte, error) {
	if err := m.Check(CategoryFS, ActionRead, path); err != nil {
		return nil, err
	}
	return defaultBackend.ReadFile(path)
}

// ReadFileAt checks fs:read:{declaredPath} against m, then reads osPath.
//
// Use this when the OS path is absolute or machine-specific (e.g. built with
// runtime.Caller to locate a file next to the source) and the capability
// should be declared using a stable, repo-relative logical identifier instead.
//
// Example:
//
//	// Manifest declares: Target: "code/grammars/verilog.tokens"
//	// lexer.go calls:
//	data, err := cage.ReadFileAt(Manifest,
//	    "code/grammars/verilog.tokens", // capability key — checked against manifest
//	    getGrammarPath(),               // actual OS path — used for the read
//	)
//
// This is the correct function for grammar-backed lexers. The declared path is
// always the same regardless of where the repo is checked out; the OS path
// is computed at runtime and varies per machine.
func ReadFileAt(m *Manifest, declaredPath, osPath string) ([]byte, error) {
	if err := m.Check(CategoryFS, ActionRead, declaredPath); err != nil {
		return nil, err
	}
	return defaultBackend.ReadFile(osPath)
}

// WriteFile checks fs:write:{path} against m, then writes data to the file.
//
// The file is created if it does not exist, or truncated if it does.
// perm sets the file permission bits (e.g., 0o644).
//
// Returns CapabilityViolationError if the manifest does not declare
// fs:write for the given path.
func WriteFile(m *Manifest, path string, data []byte, perm os.FileMode) error {
	if err := m.Check(CategoryFS, ActionWrite, path); err != nil {
		return err
	}
	return defaultBackend.WriteFile(path, data, perm)
}

// CreateFile checks fs:create:{path} against m, then creates the named file.
//
// The file must not already exist (returns an error if it does).
// Use WriteFile to create-or-truncate.
//
// Returns CapabilityViolationError if the manifest does not declare
// fs:create for the given path.
func CreateFile(m *Manifest, path string) error {
	if err := m.Check(CategoryFS, ActionCreate, path); err != nil {
		return err
	}
	return defaultBackend.CreateFile(path)
}

// DeleteFile checks fs:delete:{path} against m, then removes the named file.
//
// Returns CapabilityViolationError if the manifest does not declare
// fs:delete for the given path.
func DeleteFile(m *Manifest, path string) error {
	if err := m.Check(CategoryFS, ActionDelete, path); err != nil {
		return err
	}
	return defaultBackend.DeleteFile(path)
}

// ListDir checks fs:list:{path} against m, then returns directory entries.
//
// Returns a slice of entry names (not full paths) in the named directory.
// Returns CapabilityViolationError if the manifest does not declare
// fs:list for the given path.
func ListDir(m *Manifest, path string) ([]string, error) {
	if err := m.Check(CategoryFS, ActionList, path); err != nil {
		return nil, err
	}
	return defaultBackend.ListDir(path)
}

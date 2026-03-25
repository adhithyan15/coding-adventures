// Secure filesystem wrappers.
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
	return StartNew[[]byte]("capability-cage.ReadFile", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("path", path)
			if err := m.Check(CategoryFS, ActionRead, path); err != nil {
				return rf.Fail(nil, err)
			}
			data, err := defaultBackend.ReadFile(path)
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, data)
		}).GetResult()
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
func WriteFile(m *Manifest, path string, data []byte, perm os.FileMode) error {
	_, err := StartNew[struct{}]("capability-cage.WriteFile", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("path", path)
			if err := m.Check(CategoryFS, ActionWrite, path); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			if err := defaultBackend.WriteFile(path, data, perm); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// CreateFile checks fs:create:{path} against m, then creates the named file.
func CreateFile(m *Manifest, path string) error {
	_, err := StartNew[struct{}]("capability-cage.CreateFile", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("path", path)
			if err := m.Check(CategoryFS, ActionCreate, path); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			if err := defaultBackend.CreateFile(path); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// DeleteFile checks fs:delete:{path} against m, then removes the named file.
func DeleteFile(m *Manifest, path string) error {
	_, err := StartNew[struct{}]("capability-cage.DeleteFile", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("path", path)
			if err := m.Check(CategoryFS, ActionDelete, path); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			if err := defaultBackend.DeleteFile(path); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// ListDir checks fs:list:{path} against m, then returns directory entries.
func ListDir(m *Manifest, path string) ([]string, error) {
	return StartNew[[]string]("capability-cage.ListDir", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			op.AddProperty("path", path)
			if err := m.Check(CategoryFS, ActionList, path); err != nil {
				return rf.Fail(nil, err)
			}
			names, err := defaultBackend.ListDir(path)
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, names)
		}).GetResult()
}

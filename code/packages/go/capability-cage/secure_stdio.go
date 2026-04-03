// Secure standard I/O wrappers.
package capabilitycage

// ReadStdin checks stdin:read:* against m, then reads up to len(p) bytes.
func ReadStdin(m *Manifest, p []byte) (int, error) {
	return StartNew[int]("capability-cage.ReadStdin", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if err := m.Check(CategoryStdin, ActionRead, "*"); err != nil {
				return rf.Fail(0, err)
			}
			n, err := defaultBackend.ReadStdin(p)
			if err != nil {
				return rf.Fail(0, err)
			}
			return rf.Generate(true, false, n)
		}).GetResult()
}

// WriteStdout checks stdout:write:* against m, then writes p to stdout.
func WriteStdout(m *Manifest, p []byte) (int, error) {
	return StartNew[int]("capability-cage.WriteStdout", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if err := m.Check(CategoryStdout, ActionWrite, "*"); err != nil {
				return rf.Fail(0, err)
			}
			n, err := defaultBackend.WriteStdout(p)
			if err != nil {
				return rf.Fail(0, err)
			}
			return rf.Generate(true, false, n)
		}).GetResult()
}

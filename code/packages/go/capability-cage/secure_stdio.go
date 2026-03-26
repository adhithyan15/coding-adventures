// Secure standard I/O wrappers.
//
// These functions wrap stdin reading and stdout writing with capability checks.
//
// Target is always "*" for stdio operations — there is no meaningful resource
// identifier for "stdin" or "stdout" beyond the category itself.
// Packages that need stdio access declare stdin:read:* or stdout:write:*.
//
// Interactive programs (REPL loops, Brainfuck interpreters) typically declare
// both stdin:read:* and stdout:write:* in their manifest.
package capabilitycage

// ReadStdin checks stdin:read:* against m, then reads up to len(p) bytes.
//
// Returns the number of bytes read and any error encountered.
// Returns CapabilityViolationError if the manifest does not declare stdin:read.
func ReadStdin(m *Manifest, p []byte) (int, error) {
	if err := m.Check(CategoryStdin, ActionRead, "*"); err != nil {
		return 0, err
	}
	return defaultBackend.ReadStdin(p)
}

// WriteStdout checks stdout:write:* against m, then writes p to stdout.
//
// Returns the number of bytes written and any error encountered.
// Returns CapabilityViolationError if the manifest does not declare stdout:write.
func WriteStdout(m *Manifest, p []byte) (int, error) {
	if err := m.Check(CategoryStdout, ActionWrite, "*"); err != nil {
		return 0, err
	}
	return defaultBackend.WriteStdout(p)
}

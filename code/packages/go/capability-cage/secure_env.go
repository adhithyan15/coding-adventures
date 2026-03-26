// Secure environment variable wrappers.
//
// These functions wrap os.Getenv and os.Setenv with capability checks.
//
// Target is the environment variable name (e.g., "HOME", "PATH").
// Use "*" in the manifest target to permit access to any variable.
package capabilitycage

// ReadEnv checks env:read:{key} against m, then returns the variable's value.
//
// Returns the empty string if the variable is not set (matching os.Getenv
// semantics). Returns CapabilityViolationError if the manifest does not
// declare env:read for the given key.
func ReadEnv(m *Manifest, key string) (string, error) {
	if err := m.Check(CategoryEnv, ActionRead, key); err != nil {
		return "", err
	}
	return defaultBackend.Getenv(key), nil
}

// WriteEnv checks env:write:{key} against m, then sets the variable.
//
// Returns CapabilityViolationError if the manifest does not declare
// env:write for the given key.
func WriteEnv(m *Manifest, key, value string) error {
	if err := m.Check(CategoryEnv, ActionWrite, key); err != nil {
		return err
	}
	return defaultBackend.Setenv(key, value)
}

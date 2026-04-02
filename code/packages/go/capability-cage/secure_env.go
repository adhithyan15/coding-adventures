// Secure environment variable wrappers.
package capabilitycage

// ReadEnv checks env:read:{key} against m, then returns the variable's value.
func ReadEnv(m *Manifest, key string) (string, error) {
	return StartNew[string]("capability-cage.ReadEnv", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("key", key)
			if err := m.Check(CategoryEnv, ActionRead, key); err != nil {
				return rf.Fail("", err)
			}
			return rf.Generate(true, false, defaultBackend.Getenv(key))
		}).GetResult()
}

// WriteEnv checks env:write:{key} against m, then sets the variable.
func WriteEnv(m *Manifest, key, value string) error {
	_, err := StartNew[struct{}]("capability-cage.WriteEnv", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("key", key)
			if err := m.Check(CategoryEnv, ActionWrite, key); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			if err := defaultBackend.Setenv(key, value); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

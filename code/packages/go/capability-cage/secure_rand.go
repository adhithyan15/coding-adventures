// Secure randomness wrapper.
//
// rand:read:* is the capability for ambient entropy: the operating system's
// random source, or anything seeded from it. It exists so a unit test can
// swap the entropy for a seeded stream and replay the run (D21, VER00 §6.3).
//
// A generator seeded with a value the caller passes in is pure computation
// and needs no capability:
//
//	rng := mathrand.New(mathrand.NewSource(42)) // deterministic: no capability
//	n, err := capabilitycage.ReadRandom(m, key) // ambient entropy: rand:read:*
//
// rand is not a scoped category. There is one entropy source, so the only
// target is "*".
package capabilitycage

// ReadRandom checks rand:read:* against m, then fills p with random bytes
// from the backend. It returns the number of bytes read.
func ReadRandom(m *Manifest, p []byte) (int, error) {
	return StartNew[int]("capability-cage.ReadRandom", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("bytes", len(p))
			if err := m.Check(CategoryRand, ActionRead, "*"); err != nil {
				return rf.Fail(0, err)
			}
			n, err := defaultBackend.ReadRandom(p)
			if err != nil {
				return rf.Fail(n, err)
			}
			return rf.Generate(true, false, n)
		}).GetResult()
}

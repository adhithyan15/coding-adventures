// Secure thread wrapper.
//
// thread:spawn:{role} is the capability to start a concurrent unit of work
// inside this process: in Go, a goroutine. The target is the worker's role,
// not an instance, so a pool of eight decoders is one declaration:
//
//	{"category": "thread", "action": "spawn", "target": "decode-pool", ...}
//
//	for i := 0; i < 8; i++ {
//	    capabilitycage.Spawn(m, "decode-pool", decodeLoop)
//	}
//
// Routing goroutines through the backend lets a simulated host decide when
// each one runs, which is what makes a concurrent unit test replayable.
//
// thread:message has no wrapper here. A message between goroutines is a
// channel operation, and Go channels are typed values a generic Backend
// method cannot carry. The capability-cage generator refuses that pair
// rather than emit an unchecked method.
package capabilitycage

import "fmt"

// Spawn checks thread:spawn:{role} against m, then starts body concurrently
// through the backend. body must not be nil: a nil body would start a
// goroutine that panics far from the call that caused it.
func Spawn(m *Manifest, role string, body func()) error {
	_, err := StartNew[struct{}]("capability-cage.Spawn", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("role", role)
			if err := m.Check(CategoryThread, ActionSpawn, role); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			if body == nil {
				return rf.Fail(struct{}{}, fmt.Errorf("capability-cage.Spawn: nil body for role %q", role))
			}
			defaultBackend.Spawn(role, body)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// Secure time wrappers.
package capabilitycage

import "time"

// Now checks time:read:* against m, then returns the current time.
func Now(m *Manifest) (time.Time, error) {
	return StartNew[time.Time]("capability-cage.Now", time.Time{},
		func(op *Operation[time.Time], rf *ResultFactory[time.Time]) *OperationResult[time.Time] {
			if err := m.Check(CategoryTime, ActionRead, "*"); err != nil {
				return rf.Fail(time.Time{}, err)
			}
			return rf.Generate(true, false, defaultBackend.Now())
		}).GetResult()
}

// Sleep checks time:sleep:* against m, then sleeps for d.
func Sleep(m *Manifest, d time.Duration) error {
	_, err := StartNew[struct{}]("capability-cage.Sleep", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("duration", d)
			if err := m.Check(CategoryTime, ActionSleep, "*"); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			defaultBackend.Sleep(d)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// Secure time wrappers.
//
// These functions wrap time.Now and time.Sleep with capability checks.
//
// Target is always "*" for time operations — there is no meaningful
// resource identifier for "get the current time" or "sleep for N ms".
// Packages that need time access declare time:read:* or time:sleep:*.
package capabilitycage

import "time"

// Now checks time:read:* against m, then returns the current time.
//
// Returns CapabilityViolationError if the manifest does not declare time:read.
func Now(m *Manifest) (time.Time, error) {
	if err := m.Check(CategoryTime, ActionRead, "*"); err != nil {
		return time.Time{}, err
	}
	return defaultBackend.Now(), nil
}

// Sleep checks time:sleep:* against m, then sleeps for d.
//
// Returns CapabilityViolationError if the manifest does not declare time:sleep.
func Sleep(m *Manifest, d time.Duration) error {
	if err := m.Check(CategoryTime, ActionSleep, "*"); err != nil {
		return err
	}
	defaultBackend.Sleep(d)
	return nil
}

package record

// SerialBusyError is LockSerial's failure when the lock is HELD by another step:
// the only failure that says two steps overlap (decision P13), and so the only
// one that earns an `.overlap` marker.
type SerialBusyError struct{ Err error }

func (e *SerialBusyError) Error() string {
	return "traces/.serial is held by another step (recording is serial: run the build with -jobs 1): " + e.Err.Error()
}

// Unwrap returns the underlying errno.
func (e *SerialBusyError) Unwrap() error { return e.Err }

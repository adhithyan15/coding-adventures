package trace

import (
	"bytes"
	"io"
	"time"
)

// Measurement is what Measure found out about one trace.
type Measurement struct {
	Bytes    int           // size of the trace
	Lines    int           // number of lines
	OldTime  time.Duration // pass 1 with the hand-written scanner
	NewTime  time.Duration // pass 1 with the grammar path
	OldError string        // "" or the refusal reason
	NewError string
	// NewWindow is, when only the grammar path refused, a bounded, sanitised
	// window of the lines around the line it refused.
	NewWindow string
}

// Measure reads a trace (at most MaxTotalBytes of it) and times pass 1, the line
// reader, with each implementation.  It exists for D24's default-flip gate
// (KILN04 section 4.9): the kiln-sandbox CI job logs the sizes of the traces real
// strace produces and the grammar path's throughput on them, so the gate is
// stated from measurements.  It changes nothing and returns nothing a caller can
// trust as a parse: use Parse for that.
func Measure(r io.Reader, helperRoot bool) (Measurement, error) {
	lim := DefaultLimits()
	data, err := io.ReadAll(io.LimitReader(r, lim.MaxTotalBytes))
	if err != nil {
		return Measurement{}, err
	}
	m := Measurement{Bytes: len(data), Lines: bytes.Count(data, []byte{'\n'})}
	t0 := time.Now()
	_, _, e := readRecords(bytes.NewReader(data), lim, helperRoot, false)
	m.OldTime = time.Since(t0)
	if e != nil {
		m.OldError = e.Reason
	}
	t0 = time.Now()
	_, _, e, at := readRecordsAt(bytes.NewReader(data), lim, helperRoot, true)
	m.NewTime = time.Since(t0)
	if e != nil {
		m.NewError = e.Reason
		m.NewWindow = sanitisedWindow(data, at)
	}
	return m, nil
}

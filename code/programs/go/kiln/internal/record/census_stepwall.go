package record

// Per-language step wall time and unknown-write-set reason codes
// (KILN04, "`kiln record census`", census PR C)
// ====================================================================
//
// Two questions a census run could not answer before:
//
//  1. "Which language's steps were slow?"  One census lost a whole run to cargo
//     silently retrying the network for about 589 s per Rust step.  The summary
//     had a single blended step-wall distribution, so the stall looked like slow
//     compilation.  Package fetchers for other ecosystems (NuGet, bundler,
//     pip/uv, cabal) can stall the same way.  A per-language wall makes the odd
//     one out visible at a glance.
//
//  2. "Why is this action's write set unknown?"  collect writes a free-text
//     reason into a record's `unknown_writes` field.  The census maps the text
//     before the first colon to one of a few FIXED codes, so the summary can
//     group them without ever copying free text (the rule of census_explain.go).
//
// Where the durations come from.  They are NOT newly measured or parsed: step()
// already derives each step's wall from the step sidecar's started_at and
// ended_at stamps (the authoritative record of the traced run) for the blended
// distribution.  This file only also files that same number under the action's
// language, which the census already resolved (censusRun.languageAcc).

import (
	"bufio"
	"bytes"
	"encoding/json"
	"errors"
	"io"
	"strings"
)

// LangWall is one language's step wall time, in seconds.  N is the number of
// steps whose wall was positive and parsed (the others are counted once, in
// timing.walls_dropped, not here).  P50 is the nearest-rank median, so with an
// even count it is the LOWER middle sample (n = 4: the 2nd), with an odd count
// the middle one, with one sample that sample, with none 0.
//
// Sum is the total seconds, so a language's share of the build's wall clock can
// be read off directly.
type LangWall struct {
	N   int     `json:"n"`
	P50 float64 `json:"p50"`
	Max float64 `json:"max"`
	Sum float64 `json:"sum"`
}

// langWallOf summarises step walls given in MILLISECONDS as seconds, to three
// decimals.  It does not modify its argument.
func langWallOf(ms []float64) LangWall {
	d := distOf(ms) // sorted copy, nearest-rank, rounded to 3 decimals
	var sum float64
	for _, v := range ms {
		sum += v
	}
	return LangWall{N: d.N, P50: round3(d.P50 / 1000), Max: round3(d.Max / 1000), Sum: round3(sum / 1000)}
}

// Fixed codes of an unknown-write-set reason.  The set is closed and written
// here; `other` catches everything else, so the number of keys is bounded.
const (
	UnknownSerialViolated = "serial-violated"
	UnknownInterrupted    = "interrupted"
	UnknownNotRun         = "not-run"
	UnknownExecHeadSeal   = "exechead-seal" // D48 step 6: a started step disagrees with the sealed exec-head choice
	UnknownOther          = "other"
)

// unknownCodes is every key of CollectSummary.UnknownReasons, in print order.
var unknownCodes = []string{UnknownSerialViolated, UnknownInterrupted, UnknownNotRun, UnknownExecHeadSeal, UnknownOther}

// unknownReasonCode maps a record's `unknown_writes` text to a fixed code by its
// prefix up to the first colon:
//
//	serial-violated: seq 3 was used twice        -> serial-violated
//	interrupted: a step started and was ...      -> interrupted
//	not run: the build stopped before this step  -> not-run
//	exechead-seal: a started step disagrees ...  -> exechead-seal
//	anything else, or no colon at all            -> other
//
// These are plain string prefixes, so the "interrupted" and "not run" ones work
// whether or not the collect change that emits them is on this branch.  The text
// itself is never copied.
func unknownReasonCode(s string) string {
	prefix := s
	if i := strings.IndexByte(s, ':'); i >= 0 {
		prefix = s[:i]
	}
	switch prefix {
	case "serial-violated":
		return UnknownSerialViolated
	case "interrupted":
		return UnknownInterrupted
	case "not run":
		return UnknownNotRun
	case "exechead-seal":
		return UnknownExecHeadSeal
	}
	return UnknownOther
}

// scanRecordLines reads the record lines of records.jsonl after the header from
// r, counting each newline-terminated line and, for each line that carries a
// non-empty `unknown_writes`, counting its reason code in codes.  Reading stops
// after at most limit bytes.  A line longer than the 1 MiB buffer is counted but
// not decoded.  The file is untrusted: a line that does not decode is simply not
// given a code.
func scanRecordLines(r io.Reader, limit int64, codes map[string]int) int64 {
	if limit < 0 {
		limit = 0
	}
	br := bufio.NewReaderSize(io.LimitReader(r, limit), censusRecordsReadChunk)
	var n int64
	skipping := false // inside an over-long line: drop it up to its newline
	for {
		line, err := br.ReadSlice('\n')
		switch {
		case err == nil:
			n++
			if !skipping {
				noteUnknown(line, codes)
			}
			skipping = false
		case errors.Is(err, bufio.ErrBufferFull):
			skipping = true // counted when its newline arrives
		default:
			return n // EOF: an unterminated last line is not counted
		}
	}
}

// noteUnknown counts one record line's unknown-write-set reason, if it has one.
func noteUnknown(line []byte, codes map[string]int) {
	// A precheck that only saves CPU: the encoder emits no spaces, so a line
	// with a non-empty reason holds these bytes (an empty one holds
	// `"unknown_writes":""`, which does not match, and is skipped without a
	// decode).  The precheck is NOT what decides anything: a nested object may
	// hold the same bytes, and a key may be spelled differently.  Correctness
	// comes from the decode below, which reads the top-level field only.
	if !bytes.Contains(line, []byte(`"unknown_writes":"`)) {
		return
	}
	var rec struct {
		Unknown string `json:"unknown_writes"`
	}
	if json.Unmarshal(line, &rec) != nil || rec.Unknown == "" {
		return
	}
	codes[unknownReasonCode(rec.Unknown)]++
}

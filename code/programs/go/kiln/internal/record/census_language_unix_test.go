//go:build unix

package record

// The census's parse refusals by language and by system call (KILN04, census
// section): end to end on the canned record directory, with a stub cross-check
// that refuses with a chosen *trace.Error, so no host behaviour is involved.

import (
	"errors"
	"io"
	"os"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// record.json's parse reasons are counted under the action's language, once per
// action per code, and the overall table is their sum.
func TestCensusParseReasonsByLanguage(t *testing.T) {
	f := censusDirFix(t)
	putRecord(t, f, actA, func(r *ClassifiedRecord) {
		r.ParseOK, r.Record, r.WritesComplete = false, nil, false
		r.ParseReasons = []string{"step 0: trace-incomplete:cwd-race", "step 1: trace-incomplete:cwd-race", "step 1: trace-incomplete:fd-path"}
	})
	putRecord(t, f, actZ, func(r *ClassifiedRecord) {
		r.ParseOK, r.Record, r.WritesComplete = false, nil, false
		r.ParseReasons = []string{"step 0: trace-incomplete:cwd-race", "step 0: trace-incomplete:refused:ptrace"}
	})
	_, s, _, mdPath := runCensus(t, f)
	if len(s.Languages) != 1 {
		t.Fatalf("languages: %+v", s.Languages)
	}
	lang := s.Languages[0].Language
	ex := s.Explain
	got := ex.RecordParseReasonsByLanguage[lang]
	if got["cwd-race"] != 2 || got["fd-path"] != 1 || got["refused:ptrace"] != 1 || len(got) != 3 || len(ex.RecordParseReasonsByLanguage) != 1 {
		t.Errorf("by language: %v", ex.RecordParseReasonsByLanguage)
	}
	for code, n := range ex.RecordParseReasons {
		if got[code] != n {
			t.Errorf("%s: overall %d, by language %d", code, n, got[code])
		}
	}
	md, _ := os.ReadFile(mdPath)
	if !strings.Contains(string(md), "`"+lang+"`: `cwd-race` 2;") {
		t.Errorf("the Markdown does not show the per-language reasons:\n%s", md)
	}
}

// The census's own refusals carry the call: cwd-race says chdir, fd-path says
// fchmod, refused:<name> says its name, and anything the table does not know, or
// no call at all, says other.  The call is looked up in the closed table, so a
// hostile name never reaches the report.
func TestCensusParseRefusalSyscalls(t *testing.T) {
	type parse = func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, trace.CrossCheckDetail)
	refuse := func(errs ...error) parse {
		n := 0
		return func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, trace.CrossCheckDetail) {
			e := errs[n%len(errs)]
			n++
			return nil, e, nil, trace.CrossCheckDetail{}
		}
	}
	f := censusDirFix(t)
	_, s, _, mdPath := runCensus(t, f, func(o *CensusRunOptions) {
		o.CrossCheck = refuse(
			&trace.Error{Reason: "cwd-race", Call: "chdir"},
			&trace.Error{Reason: "fd-path", Call: "fchmod"},
		)
	})
	ex := s.Explain
	if ex.ParseRefusalSyscalls["cwd-race"]["chdir"] != 1 || ex.ParseRefusalSyscalls["fd-path"]["fchmod"] != 1 || len(ex.ParseRefusalSyscalls) != 2 {
		t.Errorf("reason x syscall: %v", ex.ParseRefusalSyscalls)
	}
	lang := s.Languages[0].Language
	if ex.ParseRefusalSyscallsByLanguage[lang]["cwd-race"]["chdir"] != 1 {
		t.Errorf("by language: %v", ex.ParseRefusalSyscallsByLanguage)
	}
	md, _ := os.ReadFile(mdPath)
	if !strings.Contains(string(md), "`cwd-race`: `chdir` 1;") {
		t.Errorf("the Markdown does not show reason x syscall:\n%s", md)
	}

	// The fallbacks.
	f = censusDirFix(t)
	_, s, _, _ = runCensus(t, f, func(o *CensusRunOptions) {
		o.CrossCheck = refuse(
			&trace.Error{Reason: "cwd-race", Call: "evil\n::error::x"},
			&trace.Error{Reason: "refused:ptrace"},
		)
	})
	ex = s.Explain
	if ex.ParseRefusalSyscalls["cwd-race"]["other"] != 1 || ex.ParseRefusalSyscalls["refused:ptrace"]["ptrace"] != 1 {
		t.Errorf("fallbacks: %v", ex.ParseRefusalSyscalls)
	}
	f = censusDirFix(t)
	_, s, _, _ = runCensus(t, f, func(o *CensusRunOptions) {
		o.CrossCheck = refuse(errors.New("not a trace error"), &trace.Error{Reason: "unparsable-line"})
	})
	if got := s.Explain.ParseRefusalSyscalls; got["unparsable-line"]["other"] != 1 || got["other"]["other"] != 1 {
		t.Errorf("no call: %v", got)
	}
}

// The timed parse's refusals are split by language too.
func TestCensusParseRefusalsByLanguage(t *testing.T) {
	f := censusDirFix(t)
	// Truncate A's trace so the real parse refuses it.
	if err := os.WriteFile(StepFile(f.p.dir, actA, 0, "trace"), []byte("garbage that is no strace line\n"), 0o644); err != nil {
		t.Skipf("cannot rewrite the trace: %v", err)
	}
	_, s, _, _ := runCensus(t, f)
	total := 0
	for _, n := range s.Explain.ParseRefusals {
		total += n
	}
	byLang := 0
	for _, m := range s.Explain.ParseRefusalsByLanguage {
		for _, n := range m {
			byLang += n
		}
	}
	if total == 0 || byLang != total {
		t.Errorf("refusals %v, by language %v", s.Explain.ParseRefusals, s.Explain.ParseRefusalsByLanguage)
	}
}

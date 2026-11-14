//go:build unix

package record

// Tests of `kiln record collect`, part 2: the per-action verdict (KILN04 section
// 4.8, P12 and P13). One table breaks ONE thing in an otherwise clean run of the
// action x/a and requires that exactly that action is an unknown write set, with
// a reason that names the cause, while x/b and x/z stay believed. The other
// actions staying known matters: a fault is charged to the action that has it.

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

func TestCollectVerdictTable(t *testing.T) {
	for _, c := range []struct {
		name string
		do   func(t *testing.T, f *cfix)
		want string
	}{
		{"record.failed alone", func(t *testing.T, f *cfix) {
			os.Remove(RecordFile(f.p.dir, ca))
			os.WriteFile(RecordFailedFile(f.p.dir, ca), []byte("timeout: classification took too long"), 0o600)
		}, "record.failed: timeout"},
		{"record.failed overrides a good record.json", func(t *testing.T, f *cfix) {
			os.WriteFile(RecordFailedFile(f.p.dir, ca), []byte("panic: boom"), 0o600)
		}, "the failure overrides it"},
		{"an empty record.failed", func(t *testing.T, f *cfix) {
			os.Remove(RecordFile(f.p.dir, ca))
			os.WriteFile(RecordFailedFile(f.p.dir, ca), nil, 0o600)
		}, "no usable record: record.failed"},
		{"record.failed is not readable", func(t *testing.T, f *cfix) {
			os.Remove(RecordFile(f.p.dir, ca))
			os.Symlink("/etc/hostname", RecordFailedFile(f.p.dir, ca))
		}, "record.failed (unreadable"},
		{"a missing record.json for an action that ran", func(t *testing.T, f *cfix) {
			os.Remove(RecordFile(f.p.dir, ca))
		}, "no record.json for an action that ran"},
		{"a record.json that does not decode", func(t *testing.T, f *cfix) {
			os.WriteFile(RecordFile(f.p.dir, ca), []byte("{"), 0o600)
		}, "record.json does not decode"},
		{"a stray record.json.tmp", func(t *testing.T, f *cfix) {
			os.WriteFile(RecordFile(f.p.dir, ca)+".tmp-4242", []byte("{}"), 0o600)
		}, "a stray record.json.tmp-4242"},
		{"a stray record.failed.tmp", func(t *testing.T, f *cfix) {
			os.WriteFile(RecordFailedFile(f.p.dir, ca)+".tmp-1", nil, 0o600)
		}, "a stray record.failed.tmp-1"},
		{"a record.json of another action", func(t *testing.T, f *cfix) {
			os.Remove(RecordFile(f.p.dir, ca))
			f.verdict(ca, 1, func(c *ClassifiedRecord) { c.Action, c.Record = cb, rawFor(cb) })
		}, "record.json is of another action"},
		{"a record.json under another binding", func(t *testing.T, f *cfix) {
			os.Remove(RecordFile(f.p.dir, ca))
			f.verdict(ca, 1, func(c *ClassifiedRecord) { c.BindingID = strings.Repeat("ab", 32) })
		}, "another binding"},
		{"a record.json from another sealed directory", func(t *testing.T, f *cfix) {
			os.Remove(RecordFile(f.p.dir, ca))
			f.verdict(ca, 1, func(c *ClassifiedRecord) { c.SealedSHA256 = strings.Repeat("cd", 32) })
		}, "another sealed directory"},
		{"a record.json that counts other steps", func(t *testing.T, f *cfix) {
			os.Remove(RecordFile(f.p.dir, ca))
			f.verdict(ca, 3, nil)
		}, "counts 3 started steps, the sidecars 1"},
		{"a record.json over the size bound", func(t *testing.T, f *cfix) {
			os.Truncate(RecordFile(f.p.dir, ca), DefaultMaxRecordBytes+1)
		}, "record-too-large"},
		{"a record.json that is a symlink", func(t *testing.T, f *cfix) {
			p := RecordFile(f.p.dir, ca)
			os.Rename(p, p+".real")
			os.Symlink(p+".real", p)
		}, "record.json"},
		{"a record.json that is a directory", func(t *testing.T, f *cfix) {
			p := RecordFile(f.p.dir, ca)
			os.Remove(p)
			os.Mkdir(p, 0o700)
		}, "record.json"},
		{"a trace that did not parse", func(t *testing.T, f *cfix) {
			os.Remove(RecordFile(f.p.dir, ca))
			f.verdict(ca, 1, func(c *ClassifiedRecord) {
				c.ParseOK, c.Record, c.WritesComplete = false, nil, false
				c.ParseReasons = []string{"step 0: the root's exit line is not the sidecar's root_status"}
			})
		}, "did not pass the parse half of G7"},
		// A refused step ran nothing, but the sandbox may have failed after it
		// changed the disk: never "wrote nothing".
		{"sandbox-refused is not an empty write set", func(t *testing.T, f *cfix) {
			os.Remove(RecordFile(f.p.dir, ca))
			f.verdict(ca, 1, func(c *ClassifiedRecord) {
				c.ParseOK, c.Record, c.WritesComplete = false, nil, false
				c.ParseReasons = []string{"step 0: sandbox-refused"}
			})
		}, "sandbox-refused"},
	} {
		t.Run(c.name, func(t *testing.T) {
			f := newCfix(t)
			f.good()
			c.do(t, f)
			_, sh := f.collect()
			requireUnknown(t, sh, ca, c.want)
			requireKnown(t, sh, cb)
			requireKnown(t, sh, cz)
		})
	}
}

// What `collect` leaves of a sidecar and its trace: every way the pair can be
// wrong. Each case names the action it breaks (x/a unless target says otherwise).
func TestCollectTraceChecks(t *testing.T) {
	for _, c := range []struct {
		name, target string
		do           func(t *testing.T, f *cfix)
		want         string
	}{
		{"a trace edited after its sidecar", ca, func(t *testing.T, f *cfix) {
			p := StepFile(f.p.dir, ca, 0, "trace")
			data, _ := os.ReadFile(p)
			data[0] ^= 1
			os.WriteFile(p, data, 0o600)
		}, "does not hash to the sidecar"},
		{"a trace that grew", ca, func(t *testing.T, f *cfix) {
			p := StepFile(f.p.dir, ca, 0, "trace")
			fh, _ := os.OpenFile(p, os.O_APPEND|os.O_WRONLY, 0)
			fh.WriteString("more\n")
			fh.Close()
		}, "the trace is "},
		{"a trace cut mid line", ca, func(t *testing.T, f *cfix) {
			// A trace whose sidecar agrees with its bytes but which does not end in
			// a newline: the supervisor never writes one.
			p := StepFile(f.p.dir, ca, 0, "trace")
			data, _ := os.ReadFile(p)
			data = data[:len(data)-1]
			os.WriteFile(p, data, 0o600)
			rewriteSidecarDigest(t, f, ca, 0, data)
		}, "does not end in a newline"},
		{"an empty trace", ca, func(t *testing.T, f *cfix) {
			os.WriteFile(StepFile(f.p.dir, ca, 0, "trace"), nil, 0o600)
			rewriteSidecarDigest(t, f, ca, 0, nil)
		}, "does not end in a newline"},
		{"a trace that is a symlink", ca, func(t *testing.T, f *cfix) {
			p := StepFile(f.p.dir, ca, 0, "trace")
			os.Rename(p, p+".real")
			os.Symlink(p+".real", p)
		}, "the trace cannot be opened"},
		{"a trace that is a directory", ca, func(t *testing.T, f *cfix) {
			p := StepFile(f.p.dir, ca, 0, "trace")
			os.Remove(p)
			os.Mkdir(p, 0o700)
		}, "not a regular file"},
		{"a sidecar without a trace", ca, func(t *testing.T, f *cfix) {
			os.Remove(StepFile(f.p.dir, ca, 0, "trace"))
		}, "step 0 has a sidecar and no trace"},
		{"a trace without a sidecar", ca, func(t *testing.T, f *cfix) {
			os.Remove(StepFile(f.p.dir, ca, 0, "json"))
		}, "step 0 has a trace and no sidecar"},
		{"a step that took a number and was never accounted for", ca, func(t *testing.T, f *cfix) {
			os.WriteFile(StepFile(f.p.dir, ca, 1, "trace"), []byte("x\n"), 0o600)
		}, "step 1 has a trace and no sidecar"},
		{"a missing middle sidecar", cb, func(t *testing.T, f *cfix) {
			os.Remove(StepFile(f.p.dir, cb, 0, "json"))
			os.Remove(StepFile(f.p.dir, cb, 0, "trace"))
		}, "step 0 has no sidecar but a later one does"},
	} {
		t.Run(c.name, func(t *testing.T) {
			f := newCfix(t)
			f.good()
			c.do(t, f)
			_, sh := f.collect()
			requireUnknown(t, sh, c.target, c.want)
		})
	}
}

// rewriteSidecarDigest makes the sidecar of a step agree with the trace bytes
// given (so the test reaches the newline check and not the digest check).
func rewriteSidecarDigest(t *testing.T, f *cfix, action string, step int, data []byte) {
	t.Helper()
	p := StepFile(f.p.dir, action, step, "json")
	raw, err := os.ReadFile(p)
	if err != nil {
		t.Fatal(err)
	}
	sc, err := DecodeStepSidecar(raw)
	if err != nil {
		t.Fatal(err)
	}
	sc.TraceBytes = int64(len(data))
	sc.TraceSHA256 = sumOf(data)
	enc, err := EncodeStepSidecar(sc)
	if err != nil {
		t.Fatal(err)
	}
	os.WriteFile(p, enc, 0o600)
}

// Sidecar facts: each is something only the supervisor could say and `collect`
// re-checks.
func TestCollectSidecarChecks(t *testing.T) {
	for _, c := range []struct {
		name string
		edit func(*StepSidecar)
		want string
	}{
		{"another action", func(s *StepSidecar) { s.Action = cb }, "another action or step"},
		{"another step count", func(s *StepSidecar) { s.StepCount = 2 }, "counts 2 steps, the plan has 1"},
		{"another binding", func(s *StepSidecar) { s.BindingID = strings.Repeat("ee", 32) }, "run under another binding"},
		{"a line the plan does not have", func(s *StepSidecar) { s.LineInPlan = false }, "recipe-mismatch"},
		{"a line digest that is not the plan's", func(s *StepSidecar) { s.LineSHA256 = strings.Repeat("00", 32) }, "recipe-mismatch"},
		{"trace-write-failed", func(s *StepSidecar) {
			s.Truncated, s.TruncatedReason, s.Clean, s.Unclean = true, TruncWriteFailed, false, []string{"truncated:trace-write-failed"}
		}, "trace-write-failed"},
		{"strace-pipes-held-open", func(s *StepSidecar) { s.Clean, s.Unclean = false, []string{UncleanPipesHeld} }, "strace-pipes-held-open"},
	} {
		t.Run(c.name, func(t *testing.T) {
			f := newCfix(t)
			f.good()
			f.step(ca, 0, 0, c.edit)
			_, sh := f.collect()
			r := recOf(t, sh, ca)
			if r.Unknown == "" || !strings.Contains(r.Unknown, c.want) {
				t.Fatalf("unknown_writes %q does not name %q", r.Unknown, c.want)
			}
			requireKnown(t, sh, cb)
		})
	}
	// A sidecar that does not decode, is not a regular file or is too big.
	for name, do := range map[string]func(f *cfix){
		"garbage":   func(f *cfix) { os.WriteFile(StepFile(f.p.dir, ca, 0, "json"), []byte("nope"), 0o600) },
		"oversized": func(f *cfix) { os.Truncate(StepFile(f.p.dir, ca, 0, "json"), maxSidecarBytes+1) },
		"a symlink": func(f *cfix) {
			p := StepFile(f.p.dir, ca, 0, "json")
			os.Rename(p, p+".real")
			os.Symlink(p+".real", p)
		},
	} {
		t.Run("sidecar "+name, func(t *testing.T) {
			f := newCfix(t)
			f.good()
			do(f)
			_, sh := f.collect()
			requireUnknown(t, sh, ca, "step 0")
		})
	}
}

// Markers and the files nobody should have left.
func TestCollectMarkersAndOrphans(t *testing.T) {
	for _, c := range []struct {
		name string
		do   func(f *cfix)
		want string
	}{
		{"a .rerun marker", func(f *cfix) { os.WriteFile(StepFile(f.p.dir, ca, 0, "rerun"), nil, 0o600) }, "run twice or out of order"},
		{"an unexpected file", func(f *cfix) { os.WriteFile(filepath.Join(f.dirOf(ca), "notes.txt"), nil, 0o600) }, "an unexpected entry notes.txt"},
		{"a step file with an unknown extension", func(f *cfix) { os.WriteFile(filepath.Join(f.dirOf(ca), "0.core"), nil, 0o600) }, "an unexpected entry 0.core"},
		{"a non-canonical step number", func(f *cfix) { os.WriteFile(filepath.Join(f.dirOf(ca), "00.json"), nil, 0o600) }, "an unexpected entry 00.json"},
		{"a marker that is a directory", func(f *cfix) { os.Mkdir(StepFile(f.p.dir, ca, 0, "rerun"), 0o700) }, "not a regular file"},
		{"more steps than the plan has", func(f *cfix) {
			for _, ext := range []string{"json", "trace"} {
				data, _ := os.ReadFile(StepFile(f.p.dir, ca, 0, ext))
				os.WriteFile(StepFile(f.p.dir, ca, 1, ext), data, 0o600)
			}
		}, "2 steps ran, the plan has 1"},
	} {
		t.Run(c.name, func(t *testing.T) {
			f := newCfix(t)
			f.good()
			c.do(f)
			_, sh := f.collect()
			r := recOf(t, sh, ca)
			if r.Unknown == "" {
				t.Fatal("not unknown")
			}
			if !strings.Contains(r.Unknown, c.want) {
				t.Fatalf("%q does not name %q", r.Unknown, c.want)
			}
		})
	}
	// An action directory that is a symlink is not a real directory.
	t.Run("a symlinked action directory", func(t *testing.T) {
		f := newCfix(t)
		f.good()
		d := f.dirOf(ca)
		os.Rename(d, d+".real")
		os.Symlink(d+".real", d)
		_, sh := f.collect()
		requireUnknown(t, sh, ca, "not a real directory")
	})
	// strace's own words are opaque: never opened, never in the shard.
	t.Run("strace-stderr is never read", func(t *testing.T) {
		f := newCfix(t)
		f.good()
		p := StepFile(f.p.dir, ca, 0, "strace-stderr")
		os.WriteFile(p, []byte("::error::injected workflow command\n"), 0o000)
		f.collect()
		data, _ := os.ReadFile(f.out)
		if strings.Contains(string(data), "injected") || strings.Contains(string(data), "::error") {
			t.Fatalf("strace's words reached the shard: %s", data)
		}
	})
}

func TestCollectSeqChecks(t *testing.T) {
	// Within an action the numbers must be consecutive: another action's step
	// ran in the middle (the serial lock did not hold). Only that action is
	// charged; the global rules still hold.
	t.Run("a gap inside an action", func(t *testing.T) {
		f := newCfix(t)
		f.good()
		f.step(cb, 1, 3, nil)
		f.step(cz, 0, 2, nil)
		_, sh := f.collect()
		requireUnknown(t, sh, cb, "serial-violated: step 1 has seq 3 after seq 1")
		requireKnown(t, sh, ca)
	})
	for _, c := range []struct {
		name string
		do   func(f *cfix)
		want string
	}{
		{"a number used twice", func(f *cfix) { f.step(cz, 0, 2, nil) }, "was used twice"},
		{"a number at or above the final counter", func(f *cfix) { f.setSeq("3\n") }, "not below the final counter 3"},
		{"a counter that ran ahead of the sidecars", func(f *cfix) { f.setSeq("9\n") }, "the counter reached 9 and 4 sidecars"},
		{"a missing counter", func(f *cfix) { os.Remove(filepath.Join(f.p.dir, "traces", ".seq")) }, "traces/.seq"},
		{"a counter that is not a counter", func(f *cfix) { f.setSeq("four\n") }, "not a counter"},
		{"a counter with no newline", func(f *cfix) { f.setSeq("4") }, "not a counter"},
		{"an empty counter", func(f *cfix) { f.setSeq("") }, "not a counter"},
		{"an unexpected entry in traces", func(f *cfix) { os.Mkdir(filepath.Join(f.p.dir, "traces", "stray"), 0o700) }, "unexpected entry stray"},
		{"an overlap marker", func(f *cfix) { os.WriteFile(StepFile(f.p.dir, cz, 0, "overlap"), nil, 0o600) }, "serial-violated"},
	} {
		// Every one of these makes EVERY record of the shard unknown: the order is
		// what lets an action be classified on the disk it left.
		t.Run(c.name, func(t *testing.T) {
			f := newCfix(t)
			f.good()
			c.do(f)
			_, sh := f.collect()
			for _, a := range []string{ca, cb, cz} {
				r := recOf(t, sh, a)
				if r.Unknown == "" {
					t.Fatalf("%s is still believed", a)
				}
			}
			if r := recOf(t, sh, ca); !strings.Contains(r.Unknown, c.want) {
				t.Fatalf("%q does not name %q", r.Unknown, c.want)
			}
		})
	}
}

func TestCollectOverlapVoidsEveryRecord(t *testing.T) {
	f := newCfix(t)
	f.good()
	// The refused step left only a marker and ran nothing (it is not evidence that
	// a step ran), yet the lock was held by another step: the shard is void.
	os.WriteFile(StepFile(f.p.dir, ca, 1, "overlap"), nil, 0o600)
	res, sh := f.collect()
	if len(res.Unknown) != 3 {
		t.Fatalf("%v", res.Unknown)
	}
	for _, a := range []string{ca, cb, cz} {
		requireUnknown(t, sh, a, "serial-violated")
	}
}

// The build tool's count and status are cross-checked with what the supervisor
// accounted for.
func TestCollectResultsCrossChecks(t *testing.T) {
	built := func(a string, n int) ResultsPackage {
		return ResultsPackage{Package: a, Status: "built", StepsRun: n, StepsTotal: n}
	}
	for _, c := range []struct {
		name    string
		results []ResultsPackage
		want    string
	}{
		{"the build tool started fewer steps", []ResultsPackage{built(ca, 1), {Package: cb, Status: "failed", StepsRun: 1, StepsTotal: 2}, built(cz, 1)}, "the build tool started 1 steps, 2 have a traced run"},
		{"another step total", []ResultsPackage{built(ca, 1), {Package: cb, Status: "built", StepsRun: 2, StepsTotal: 3}, built(cz, 1)}, "the build tool says 3 steps, the plan has 2"},
		{"an action the results do not list", []ResultsPackage{built(ca, 1), built(cz, 1)}, "do not list"},
	} {
		t.Run(c.name, func(t *testing.T) {
			f := newCfix(t)
			f.good()
			f.setResults(c.results...)
			_, sh := f.collect()
			requireUnknown(t, sh, cb, c.want)
			requireKnown(t, sh, ca)
		})
	}
	// built, but a step exited non-zero.
	t.Run("built with a failed step", func(t *testing.T) {
		f := newCfix(t)
		f.good()
		f.step(cb, 1, 2, func(s *StepSidecar) { s.StraceExit, s.RootStatus = 3, 3 })
		_, sh := f.collect()
		requireUnknown(t, sh, cb, "built, but a step exited non-zero")
	})
	// A failed action keeps its status: only a truncated execution for the merge.
	t.Run("a failed action is a failed record", func(t *testing.T) {
		f := newCfix(t)
		f.good()
		f.step(cb, 1, 2, func(s *StepSidecar) { s.StraceExit, s.RootStatus = 2, 2 })
		f.setResults(built(ca, 1), ResultsPackage{Package: cb, Status: "failed", StepsRun: 2, StepsTotal: 2}, built(cz, 1))
		_, sh := f.collect()
		r := requireKnown(t, sh, cb)
		if r.Status != "failed" || r.StepsRun != 2 {
			t.Fatalf("%+v", r)
		}
		// The merge charges it as a truncated execution: strict, run-void.
		m := mergeOf(f, sh)
		if m.Emitted {
			t.Fatal("a failed action must not be emitted by the strict verdict")
		}
	})
}

// A record that ran nothing is explicit false and is not an unknown write set.
func TestCollectRanNothingIsExplicitFalse(t *testing.T) {
	f := newCfix(t)
	f.good()
	// z is gated away by the build tool: no step, no directory.
	os.RemoveAll(f.dirOf(cz))
	f.setSeq("3\n")
	f.setResults(
		ResultsPackage{Package: ca, Status: "built", StepsRun: 1, StepsTotal: 1},
		ResultsPackage{Package: cb, Status: "built", StepsRun: 2, StepsTotal: 2},
		ResultsPackage{Package: cz, Status: "dep-skipped", StepsRun: 0, StepsTotal: 1})
	_, sh := f.collect()
	z := requireKnown(t, sh, cz)
	if z.StepsRun != 0 || z.Status != "dep-skipped" || z.Complete == nil || *z.Complete || len(z.Steps) != 0 {
		t.Fatalf("%+v", z)
	}
	// An action the results omit entirely is "not-run".
	f.setResults(
		ResultsPackage{Package: ca, Status: "built", StepsRun: 1, StepsTotal: 1},
		ResultsPackage{Package: cb, Status: "built", StepsRun: 2, StepsTotal: 2})
	os.Remove(f.out)
	_, sh = f.collect()
	if z := requireKnown(t, sh, cz); z.Status != "not-run" {
		t.Fatalf("%+v", z)
	}
	// Built with nothing traced is not a record anybody classified.
	f.setResults(
		ResultsPackage{Package: ca, Status: "built", StepsRun: 1, StepsTotal: 1},
		ResultsPackage{Package: cb, Status: "built", StepsRun: 2, StepsTotal: 2},
		ResultsPackage{Package: cz, Status: "built", StepsRun: 0, StepsTotal: 1})
	os.Remove(f.out)
	_, sh = f.collect()
	requireUnknown(t, sh, cz, "built without a traced step")
}

// writes_complete is the classifier's own answer, carried through CompleteOf.
func TestCollectWritesCompleteIsTheClassifiers(t *testing.T) {
	for _, want := range []bool{true, false} {
		f := newCfix(t)
		f.good()
		os.Remove(RecordFile(f.p.dir, ca))
		f.verdict(ca, 1, func(c *ClassifiedRecord) { c.WritesComplete = want })
		res, sh := f.collect()
		r := requireKnown(t, sh, ca)
		if r.Complete == nil || *r.Complete != want {
			t.Fatalf("classifier said %v, shard says %v", want, r.Complete)
		}
		// A classifier that says false is an unknown write set to the merge,
		// whatever else the record says.
		if reported := strings.Contains(strings.Join(res.Unknown, "\n"), ca+": writes_complete is false"); reported == want {
			t.Fatalf("the report says %v for a classifier that said %v: %v", reported, want, res.Unknown)
		}
		if !want {
			if m := mergeOf(f, sh); m.Emitted {
				t.Fatal("writes_complete false must void the run")
			}
		}
	}
}

// Text from the trace never reaches the shard unbounded or unprintable.
func TestCollectUnknownTextIsBounded(t *testing.T) {
	f := newCfix(t)
	f.good()
	os.Remove(RecordFile(f.p.dir, ca))
	nasty := "panic: \n::set-output name=x::y\x1b[31m‮" + strings.Repeat("é", 5000)
	os.WriteFile(RecordFailedFile(f.p.dir, ca), []byte(nasty), 0o600)
	_, sh := f.collect()
	r := recOf(t, sh, ca)
	if len(r.Unknown) > 200 || strings.ContainsAny(r.Unknown, "\n\x1b‮") {
		t.Fatalf("unknown_writes is not bounded and printable: %q", r.Unknown)
	}
	// A cut inside a multibyte rune leaves valid UTF-8.
	if !strings.HasPrefix(r.Unknown, "no usable record: record.failed: panic: ") {
		t.Fatalf("%q", r.Unknown)
	}
}

// A record the shard format cannot hold is an unknown write set, never a crash.
func TestCollectAnUnencodableRecordIsUnknown(t *testing.T) {
	f := newCfix(t)
	f.good()
	os.Remove(RecordFile(f.p.dir, ca))
	f.verdict(ca, 1, func(c *ClassifiedRecord) { c.Record.Writes = []string{"a/../b"} })
	_, sh := f.collect()
	requireUnknown(t, sh, ca, "record-unencodable")
	requireKnown(t, sh, cb)
}

// A shard over the format's size caps says so, with every record unknown, and a
// shard that cannot be made to decode at all is not written.
func TestCollectAShardOverTheCapsIsReplacedByAVoidOne(t *testing.T) {
	old := recfmt.DefaultLimits
	t.Cleanup(func() { recfmt.DefaultLimits = old })

	f := newCfix(t)
	f.good()
	os.Remove(RecordFile(f.p.dir, ca))
	f.verdict(ca, 1, func(c *ClassifiedRecord) {
		for i := 0; i < 40; i++ {
			c.Record.Writes = append(c.Record.Writes, "code/packages/x/a/out/"+strings.Repeat("d", 60)+string(rune('a'+i/26))+string(rune('a'+i%26)))
		}
	})
	recfmt.DefaultLimits.MaxLine = 1800
	res, sh := f.collect()
	if len(sh.Header.Void) != 1 || sh.Header.Void[0] != "records-too-large" || len(res.Unknown) != 3 {
		t.Fatalf("%+v %v", sh.Header, res.Unknown)
	}
	for _, a := range []string{ca, cb, cz} {
		requireUnknown(t, sh, a, "records-too-large")
	}

	g := newCfix(t)
	g.good()
	recfmt.DefaultLimits.MaxLine = 10
	g.refuse("does not decode even with every record unknown")
}

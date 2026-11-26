package record

// Rendering the census summary, and running the command (KILN04, "`kiln record
// census`")
// ==========================================================================
//
// Two outputs, both bounded:
//
//	JSON      schema kiln-census-summary/1, at most 512 KiB, from encoding/json
//	Markdown  at most 256 KiB, for $GITHUB_STEP_SUMMARY
//
// A bound is met by cutting the long lists (the top paths and prefixes of each
// kind), halving them until the document fits, and SAYING so in the report
// (bounds.json_lists_cut, bounds.md_truncated), not by silently dropping.
//
// The Markdown rule that matters: attacker text (a path, an action name, a
// mismatch detail) appears ONLY inside a code span (codeSpan) in a list item,
// never in a table cell and never as bare prose; everything else we print is a
// number, a fixed word of ours, or a validated name that is escaped anyway.

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
)

// limited returns a copy of s whose per-kind lists are cut to n entries.
func (s *CensusSummary) limited(n int) *CensusSummary {
	c := *s
	c.Reasons = make([]KindSummary, len(s.Reasons))
	for i, k := range s.Reasons {
		if len(k.TopPaths) > n {
			k.TopPaths = k.TopPaths[:n]
		}
		if len(k.Prefixes) > n {
			k.Prefixes = k.Prefixes[:n]
		}
		c.Reasons[i] = k
	}
	c.Bounds.TopPerKind = n
	return &c
}

// topLadder are the list lengths tried, longest first.
var topLadder = []int{CensusTopN, 100, 50, 20, 10, 5, 1, 0}

// EncodeCensusSummary renders s as JSON (compact, one trailing newline) within
// MaxSummaryJSONBytes, cutting the per-kind lists as far as it takes.  The
// default encoder escapes <, > and & as < and friends, so the text is safe
// even if it is ever embedded in HTML.
func EncodeCensusSummary(s *CensusSummary) ([]byte, error) {
	for _, n := range topLadder {
		c := s.limited(n)
		c.Bounds.JSONListsCut = n < CensusTopN
		data, err := json.Marshal(c)
		if err != nil {
			return nil, err
		}
		data = append(data, '\n')
		if len(data) <= MaxSummaryJSONBytes {
			return data, nil
		}
	}
	return nil, errors.New("the summary does not fit in its JSON bound even without any path list")
}

// RenderCensusMarkdown renders s as Markdown within MaxSummaryMDBytes.  It
// reports the list length it settled on and whether it had to cut lines.
func RenderCensusMarkdown(s *CensusSummary) (md string, top int, truncated bool) {
	for _, n := range topLadder {
		out := finishMarkdown(renderMarkdown(s.limited(n)))
		if len(out) <= MaxSummaryMDBytes {
			return out, n, false
		}
	}
	// Even without path lists: cut at a line boundary and say so.
	out := finishMarkdown(renderMarkdown(s.limited(0)))
	cut := out[:MaxSummaryMDBytes-256]
	if i := strings.LastIndexByte(cut, '\n'); i > 0 {
		cut = cut[:i]
	}
	return cut + "\n\n(The rest of this report was cut to fit its size bound.)\n", 0, true
}

// num formats a float without trailing noise.
func num(f float64) string { return strconv.FormatFloat(f, 'f', -1, 64) }

// pct formats a 0..1 share as a percentage.
func pct(f float64) string { return strconv.FormatFloat(f*100, 'f', 1, 64) + "%" }

func distRow(label string, d Dist, fmtv func(float64) string) string {
	return fmt.Sprintf("| %s | %d | %s | %s | %s | %s |\n", label, d.N, fmtv(d.P50), fmtv(d.P90), fmtv(d.P99), fmtv(d.Max))
}

// renderMarkdown is the document, before the final safety pass.
func renderMarkdown(s *CensusSummary) string {
	var b strings.Builder
	w := func(format string, args ...any) { fmt.Fprintf(&b, format, args...) }
	w("# Kiln record census\n\n")
	w("Schema %s. Figures are counts of what a sample recording left behind; no argv, no environment value and no trace line is in this report.\n\n", codeSpan(s.Schema))

	w("## Verdict\n\n")
	cc := s.CrossCheck
	status := "ok"
	if cc.Mismatches > 0 || cc.Allowed > 0 || s.Problems["parse_panic"] > 0 {
		status = "FAILED"
	}
	w("- cross-check: %s (traces %d, accepted %d, refused %d, mismatches %d, allowed divergences %d)\n", status, cc.Traces, cc.Accepted, cc.Refused, cc.Mismatches, cc.Allowed)
	// A trace over the cross-check size cap was timed but NOT cross-checked, so
	// "ok" above says nothing about it: put the count beside the verdict, where
	// a reader of only this section cannot miss it.
	w("- cross-check SKIPPED (trace too large to cross-check, timed only): %d traces, %d bytes (no line shape is made for such a trace either: shapes exist only for cross-checked traces)\n", cc.Skipped, cc.SkippedBytes)
	if s.InfrastructureFailure != "" {
		w("- infrastructure failure: %s\n", codeSpan(s.InfrastructureFailure))
	} else {
		w("- infrastructure failure: none\n")
	}
	w("\n")

	h := s.Host
	w("## Host\n\n- image %s; kernel %s; strace %s\n- shard %d of %d; CPU %s\n\n", codeSpan(h.ImageID), codeSpan(h.Kernel), codeSpan(h.StraceVersion), h.ShardIndex, h.ShardCount, codeSpan(h.CPUModel))

	a := s.Actions
	w("## Actions\n\n")
	w("- total: %d\n- with a record.json: %d (parse ok %d, writes complete %d)\n- record.failed: %d\n- no verdict at all: %d\n", a.Total, a.WithRecord, a.RecordParseOK, a.RecordWritesComplete, a.RecordFailed, a.NoVerdict)
	voidNote := "shard not void"
	if a.ShardVoid {
		voidNote = "SHARD VOID: collect's header names a void reason, so nothing is emitted"
	}
	w("- build did not fail (a believed record with no `failed` reason): %d\n- would emit as it stands, an UPPER BOUND (per action, before the merge: it cannot see reads-product or cross-action rules): %d (parse ok, writes complete, ZERO reasons in record.json, plan entry with no gate and a weak key, %s)\n", a.BuiltOK, a.Emittable, voidNote)
	w("- gated by the plan (never emitted; the merge makes it a precondition): %d; unkeyed (no weak key): %d\n", a.Gated, a.Unkeyed)
	w("- write set unknown (writes_complete false, or record.failed; a strict merge voids the run for any): %d\n", a.WritesUnknown)
	w("- write set known but execution truncated (failed, daemon-left, steps missing; recmerge unkTruncated): %d; the relaxed merge would lose %d action(s)", a.Truncated, a.RelaxedWouldLose)
	if a.RelaxedMergeVoidsShard {
		w(", but a hard unknown or a void shard voids it in both verdicts")
	}
	w("\n")
	yes := "no"
	if a.StrictMergeVoidsShard {
		yes = "yes"
	}
	w("- strict merge would void this shard: %s (writes unknown > 0, truncated > 0, or the shard is void)\n", yes)
	w("- census.json present and non-empty: %d\n- census.json present and empty, with a record whose parse succeeded (no reasons raised): %d\n", a.CensusNonEmpty, a.CensusClean)
	w("- census.json EMPTY beside a failed parse half (the classifier never ran; not \"no host reads\"): %d\n", a.CensusEmptyParseFailed)
	w("- census.json empty and no record: %d\n- census.json MISSING (a classify error; not \"no host reads\"): %d\n- census.json unreadable or invalid: %d\n\n", a.CensusEmptyNoRecord, a.CensusMissing, a.CensusInvalid)

	w("## Why actions are not recordable (fixed codes, no paths)\n\n")
	ex := s.Explain
	for _, g := range []struct {
		title string
		m     map[string]int
	}{
		{"classifier reason kinds in record.json (actions)", ex.RecordReasonKinds},
		{"parse reasons in record.json (actions)", ex.RecordParseReasons},
		{"parse refusals seen by this census (traces)", ex.ParseRefusals},
		{"record.failed (actions)", ex.RecordFailed},
		{"truncated steps by reason", ex.Sidecar.Truncated},
		{"tracer unclean reasons (steps)", ex.Sidecar.Unclean},
		{"strace stderr codes (steps; the text stays on the runner)", ex.Sidecar.StraceStderrCodes},
		{"strace stderr codes of steps whose build step also failed (steps)", ex.Sidecar.StraceStderrFailedSteps},
		{"strace killed by signal (steps)", ex.Sidecar.StraceSignals},
		{"strace exit status of steps with stderr text", ex.Sidecar.StraceExits},
	} {
		w("- %s:", g.title)
		if len(g.m) == 0 {
			w(" none")
		}
		for _, k := range sortedStrings(g.m) {
			w(" %s %d;", codeSpan(k), g.m[k])
		}
		w("\n")
	}
	w("- sidecars: %d steps, %d not clean, %d exit 125, %d with a non-zero root status\n\n", ex.Sidecar.Steps, ex.Sidecar.NotClean, ex.Sidecar.Exit125, ex.Sidecar.RootNonZero)

	// Parse refusals by language, and by the call they were raised for.  Every
	// name is in a code span, in a list item (see "Share by language").
	w("### Parse reasons by language\n\n")
	renderByLanguage(&b, "parse reasons in record.json (actions)", ex.RecordParseReasonsByLanguage)
	renderByLanguage(&b, "parse refusals seen by this census (traces)", ex.ParseRefusalsByLanguage)
	w("### Parse refusals by system call\n\n")
	w("Reason by the closed-table syscall the refusal was raised for (`other` when it is tied to no modelled call), from this census's cross-checked parse (traces).\n\n")
	renderReasonSyscalls(&b, "all languages", ex.ParseRefusalSyscalls)
	for _, l := range sortedMapKeys(ex.ParseRefusalSyscallsByLanguage) {
		renderReasonSyscalls(&b, l, ex.ParseRefusalSyscallsByLanguage[l])
	}
	w("\n")

	if len(s.Problems) > 0 {
		w("## Problems\n\n")
		for _, k := range sortedStrings(s.Problems) {
			w("- %s: %d\n", codeSpan(k), s.Problems[k])
		}
		w("\n")
	}

	w("## Trace sizes (bytes)\n\n| set | n | p50 | p90 | p99 | max |\n|---|---|---|---|---|---|\n")
	b.WriteString(distRow("all steps", s.TraceSizes.All.Dist, num))
	b.WriteString(distRow("steps with root_status 0", s.TraceSizes.RootOK.Dist, num))
	b.WriteString(distRow("lines per trace", s.TraceSizes.Lines, num))
	w("\n")
	if len(s.TraceSizes.All.Buckets) > 0 {
		w("Log2 buckets (all steps), each from its minimum up to twice that:\n\n")
		for _, bk := range s.TraceSizes.All.Buckets {
			w("- from %d bytes: %d traces\n", bk.Min, bk.Count)
		}
		w("\n")
	}

	t := s.Timing
	w("## Timing (G2: a reported target, not a gate)\n\n| quantity | n | p50 | p90 | p99 | max |\n|---|---|---|---|---|---|\n")
	b.WriteString(distRow("step wall (ms)", t.StepWallMS, num))
	b.WriteString(distRow("grammar parse (ms)", t.ParseMS, num))
	b.WriteString(distRow("parse share of wall", t.ParseShare, pct))
	w("\n- parse throughput (MB/s): p50 %s, p90 %s, p99 %s, max %s\n", num(t.ParseMBPerS.P50), num(t.ParseMBPerS.P90), num(t.ParseMBPerS.P99), num(t.ParseMBPerS.Max))
	w("- first (cold) parse: %s ms on %d bytes (accepted: %v); each classify child pays this initialisation again\n", num(t.FirstParse.MS), t.FirstParse.Bytes, t.FirstParse.Accepted)
	w("- G2 target (reported, not a gate): parse share p50 %s against a target of at most %s: %s; parse throughput p50 %s MB/s on image %s\n", pct(t.G2Target.ShareP50), pct(t.G2Target.TargetShareP50), map[bool]string{true: "met", false: "not met"}[t.G2Target.Met], num(t.G2Target.ParseMBPerSP50), codeSpan(t.G2Target.ImageID))
	w("- G2 at or above the p99 size (%s bytes): %d traces, share over 10%% %d, parse over 5 s %d, failing %d\n", num(t.G2AtP99.ThresholdBytes), t.G2AtP99.Traces, t.G2AtP99.Over10Pct, t.G2AtP99.Over5s, t.G2AtP99.Failing)
	for _, r := range t.BySize {
		w("- size from %d bytes: %d parses, parse ms p50 %s max %s, share p50 %s max %s\n", r.Min, r.N, num(r.ParseMS.P50), num(r.ParseMS.Max), pct(r.ParseShare.P50), pct(r.ParseShare.Max))
	}
	w("\n- parse share over 10%%: %d\n- parse over 5 s: %d\n- either old limit (G2, reported only, not a gate): %d\n- walls dropped (not positive): %d\n- parses refused (not timed): %d\n\n", t.ShareOver10Pct, t.ParseOver5s, t.G2Failing, t.WallsDropped, t.ParseRefused)

	pa := s.PerAction
	w("## Trace bytes per action (cap %d)\n\n| quantity | n | p50 | p90 | p99 | max |\n|---|---|---|---|---|---|\n", pa.CapBytes)
	b.WriteString(distRow("total trace bytes", pa.Bytes, num))
	w("\n- at least 50%% of the cap: %d\n- at least 90%% of the cap: %d\n- at or over the cap: %d\n\n", pa.Over50, pa.Over90, pa.OverCap)
	for _, ab := range pa.Largest {
		w("- %s: %d bytes (%s%% of the cap)\n", codeSpan(ab.Action), ab.Bytes, num(ab.PercentCap))
	}
	if len(pa.Largest) > 0 {
		w("\n")
	}

	w("## Cross-check\n\n- traces %d, bytes %d\n- accepted %d, refused %d\n- skipped as too large to cross-check: %d (%d bytes)\n- mismatches %d\n- allowed stricter-only divergences %d\n", cc.Traces, cc.Bytes, cc.Accepted, cc.Refused, cc.Skipped, cc.SkippedBytes, cc.Mismatches, cc.Allowed)
	// The rows' names are fixed program strings; they still go in code spans (the
	// rule of this file: no free-standing text that is not ours by construction).
	w("- allowed divergences by the spec's stricter-only row:")
	if len(cc.AllowedRows) == 0 {
		w(" none")
	}
	for _, k := range sortedStrings(cc.AllowedRows) {
		w(" %s %d;", codeSpan(k), cc.AllowedRows[k])
	}
	w("\n")
	for _, d := range cc.Details {
		w("- detail: %s\n", codeSpan(d))
	}
	w("\n")

	// The redacted outlines of the lines refusals and allowed divergences
	// stopped at.  Every token is a type name, a character class, a closed-table
	// syscall name or ASCII punctuation (trace.Shape), and still sits in a code
	// span.
	w("## Line shapes of refusals and allowed divergences (redacted)\n\nShapes exist only for cross-checked traces (not for those skipped as too large). A refusal's line is the error's own line: the call's start for an unresumed call.\n\n")
	if len(s.Shapes) == 0 {
		w("None.\n\n")
	}
	for _, sh := range s.Shapes {
		w("- %s, seen %d time(s), first at %s\n", codeSpan(sh.Origin), sh.Count, codeSpan(sh.Where))
		for _, l := range sh.Lines {
			cut := ""
			if l.Truncated {
				cut = " (cut)"
			}
			w("  - %s line %d%s: %s; facts %s\n", l.Role, l.Line, cut, codeSpan(l.Tokens), codeSpan(l.Facts))
		}
	}
	w("\n")

	// Language names are validated, but a bare name can still autolink
	// (www.evil.example) or render as an emoji (:smile:), so each is in a code
	// span, in a list item and not in a table cell.
	//
	// Wording matters here.  "recordable" is only parse ok + writes complete: it
	// says nothing about reasons, so it is NOT "would emit" (that is `emittable`:
	// recordable AND zero reasons AND not gated).  "no-reason" replaces the old
	// "clean": an empty census beside a recordable record, i.e. no host reads.
	w("## Share by language (all actions, and of builds that did not fail)\n\n")
	for _, l := range s.Languages {
		w("- %s: %d actions; %d built ok; recordable (parse ok and writes complete, NOT necessarily emittable) %d = %s of all, %s of built ok; no-reason (recordable with an empty census) %d; would emit, upper bound (zero reasons, no gate, keyed, shard not void) %d = %s of all, %s of built ok; gated %d; unkeyed %d; writes unknown %d; step wall (s) n %d, p50 %s, max %s, sum %s; failed steps (s) n %d, p50 %s, max %s, sum %s\n",
			codeSpan(l.Language), l.Actions, l.BuiltOK, l.Recordable, pct(l.Share), pct(l.RecordableShareOfBuiltOK), l.Clean, l.Emittable, pct(l.EmittableShare), pct(l.EmittableShareOfBuiltOK), l.Gated, l.Unkeyed, l.WritesUnknown, l.StepWallS.N, num(l.StepWallS.P50), num(l.StepWallS.Max), num(l.StepWallS.Sum),
			l.StepWallFailedS.N, num(l.StepWallFailedS.P50), num(l.StepWallFailedS.Max), num(l.StepWallFailedS.Sum))
	}
	w("\n")

	if c := s.Collect; c != nil {
		w("## collect\n\n- shard %d of %d, %d records\n- post-build unexplained paths: %d\n- unexplained tracked changes: %d\n", c.ShardIndex, c.ShardCount, c.Records, c.PostInventoryUnexplained, c.TrackedChangedUnexplained)
		if len(c.Void) == 0 {
			w("- void: none\n")
		}
		for _, v := range c.Void {
			w("- void: %s\n", codeSpan(v))
		}
		w("- unknown write sets by reason:")
		for _, k := range unknownCodes {
			w(" %s %d;", codeSpan(k), c.UnknownReasons[k])
		}
		w("\n")
		w("\n")
	}

	w("## traces/ markers\n\n- state-preloaded marker present: %t\n\n", s.StatePreloadedMarker)

	// Exec heads (KILN04 D48 step 6): counts only, under codes from closed lists.
	eh := s.ExecHead
	w("## Exec heads\n\n")
	if !eh.Sealed {
		w("- this shard ran WITHOUT exec heads (prepare was not given --exec-head)\n\n")
	} else {
		// The vacuity rule.  Every other number in this section is a count of
		// things going WRONG, so zero reads as success; it is only a result when
		// something that could have gone wrong happened.  A sealed shard with no Go
		// action, or in which no exec under the step tmpfs reached the pairing,
		// says little, but the two cases differ.  tmpfs_execs == 0 means the
		// mechanism itself was never exercised: its all-zero sub-codes are not a
		// result.  tmpfs_execs > 0 with no Go action (a Rust-only shard whose
		// rustdoc test binaries ran under $TMPDIR) DID exercise it; only
		// go_emittable is empty.  The line comes first so nobody reads past it.
		switch {
		case eh.TmpfsExecs == 0:
			w("%s\n\n", ExecHeadVacuousLine)
		case eh.GoActions == 0:
			w("%s\n\n", ExecHeadNoGoLine)
		}
		w("- this shard ran WITH exec heads (sealed at prepare); kernel on the trusted table: %v\n", eh.KernelTrusted)
		w("- exercised: go actions %d; go actions built ok %d; execs under the step tmpfs judged by the exec head %d\n", eh.GoActions, eh.GoBuiltOK, eh.TmpfsExecs)
		w("- supervised steps %d; supervisor notifications %d; supervisor kills %d (in %d steps)\n", eh.SupervisedSteps, eh.Notifications, eh.Kills, eh.KillSteps)
		w("- would emit as it stands, language go (an UPPER BOUND, as above): %d\n", eh.GoEmittable)
		for _, t := range []struct {
			label string
			codes []string
			m     map[string]int
		}{
			{"exec-unreadable sub-codes (occurrences)", execHeadSubCodes, eh.SubCodes},
			{"unclean steps by supervisor reason", execHeadUncleanCodes, eh.Unclean},
			{"actions refusing an exec-head file", execHeadParseCodes, eh.ParseReasons},
		} {
			w("- %s:", t.label)
			for _, c := range t.codes {
				w(" %s %d;", codeSpan(c), t.m[c])
			}
			w("\n")
		}
		w("\n")
	}

	// KILN04 D50: how often the two parse rules fired (counts only).
	w("## Parse rules\n\n")
	w("- fork, vfork or clone dropped as `= ? ERESTARTNOINTR` (rule 2b): %d\n", s.ParseRules.RestartedSpawns)
	w("- successful read-only opens accepted with no descriptor annotation (rule 2c): %d\n\n", s.ParseRules.BareOpens)

	w("## Reasons by kind\n\n")
	if len(s.Reasons) == 0 {
		w("None.\n\n")
	}
	for _, k := range s.Reasons {
		w("### Kind %s\n\n", codeSpan(k.Kind))
		w("- occurrences %d, raised in %d actions, %d distinct paths kept, overflow shed by the classifier %d, dropped here %d\n", k.Occurrences, k.Actions, k.DistinctPaths, k.Overflow, k.DroppedPaths)
		if len(k.Ops) > 0 {
			w("- by the kind of look that raised it:")
			names := make([]string, 0, len(k.Ops))
			for c := range k.Ops {
				names = append(names, c)
			}
			sort.Strings(names)
			for _, c := range names {
				w(" %s %d;", codeSpan(c), k.Ops[c])
			}
			w("\n")
		}
		if len(k.Prefixes) > 0 {
			w("\nBy path prefix (depth at most %d):\n\n", censusPrefixDepth)
			for _, p := range k.Prefixes {
				w("- %s: %d paths, %d occurrences\n", codeSpan(p.Prefix), p.Paths, p.Count)
			}
		}
		if len(k.TopPaths) > 0 {
			w("\nTop paths:\n\n")
			for _, p := range k.TopPaths {
				w("- %s: %d\n", codeSpan(p.Path), p.Count)
			}
		}
		w("\n")
	}

	bd := s.Bounds
	w("## Bounds\n\n- JSON at most %d bytes, Markdown at most %d bytes; lists in this rendering cut to %d per kind\n", bd.JSONMaxBytes, bd.MDMaxBytes, bd.TopPerKind)
	w("- paths cut to %d characters: %d; paths dropped at the distinct-path cap: %d; prefixes dropped: %d; kinds collapsed: %d; languages collapsed: %d; actions skipped: %d; line shapes not kept: %d\n", maxCensusPathRunes, bd.PathsTruncated, bd.PathsDropped, bd.PrefixesDropped, bd.KindsCollapsed, bd.LanguagesCollapsed, bd.ActionsSkipped, bd.ShapesDropped)
	return b.String()
}

// sortedMapKeys returns the keys of a map of anything, ascending.
func sortedMapKeys[V any](m map[string]V) []string {
	ks := make([]string, 0, len(m))
	for k := range m {
		ks = append(ks, k)
	}
	sort.Strings(ks)
	return ks
}

// renderByLanguage writes one list item per language under a title:
// "- title:" then "  - `lang`: `code` n; ...".
func renderByLanguage(b *strings.Builder, title string, t CountTable) {
	fmt.Fprintf(b, "- %s:", title)
	if len(t) == 0 {
		b.WriteString(" none")
	}
	b.WriteString("\n")
	for _, l := range sortedMapKeys(t) {
		fmt.Fprintf(b, "  - %s:", codeSpan(l))
		for _, k := range sortedStrings(t[l]) {
			fmt.Fprintf(b, " %s %d;", codeSpan(k), t[l][k])
		}
		b.WriteString("\n")
	}
	b.WriteString("\n")
}

// renderReasonSyscalls writes "- label:" and, under it, one item per reason
// listing its syscalls.  A table with nothing in it says none.
func renderReasonSyscalls(b *strings.Builder, label string, t CountTable) {
	fmt.Fprintf(b, "- %s:", codeSpan(label))
	if len(t) == 0 {
		b.WriteString(" none")
	}
	b.WriteString("\n")
	for _, code := range sortedMapKeys(t) {
		fmt.Fprintf(b, "  - %s:", codeSpan(code))
		for _, sys := range sortedStrings(t[code]) {
			fmt.Fprintf(b, " %s %d;", codeSpan(sys), t[code][sys])
		}
		b.WriteString("\n")
	}
}

// sortedStrings returns the keys of m, ascending.
func sortedStrings(m map[string]int) []string {
	ks := make([]string, 0, len(m))
	for k := range m {
		ks = append(ks, k)
	}
	sort.Strings(ks)
	return ks
}

// ---------------------------------------------------------------------------
// Running it
// ---------------------------------------------------------------------------

// Exit statuses of `kiln record census`.
const (
	CensusExitOK       = 0
	CensusExitInfra    = 1 // an infrastructure failure: no traces, a pre/ that does not verify, an output that could not be written
	CensusExitUsage    = 2
	CensusExitMismatch = 4 // a cross-check mismatch, an allowed divergence or a parse panic (strict)
)

// CensusRunOptions are CensusOptions and where the outputs go.
type CensusRunOptions struct {
	CensusOptions
	Out, MD string // absolute; created O_EXCL, never inside Dir
}

// CensusResult is what a run produced.
type CensusResult struct {
	Summary *CensusSummary
	Exit    int
}

// outsideDir checks that an output path is not inside dir.  The output's parent
// directory must exist; both are compared after resolving symlinks, so a link
// into the record directory does not smuggle a write there.
func outsideDir(dir, out string) error {
	cd, err := filepath.EvalSymlinks(dir)
	if err != nil {
		return err
	}
	parent, err := filepath.EvalSymlinks(filepath.Dir(out))
	if err != nil {
		return fmt.Errorf("the directory of %s does not exist", filepath.Base(out))
	}
	full := filepath.Join(parent, filepath.Base(out))
	rel, err := filepath.Rel(cd, full)
	if err == nil && rel != ".." && !strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
		return fmt.Errorf("an output may not be inside the record directory")
	}
	return nil
}

// writeNew creates path (O_EXCL: it must not exist, and a symlink at that name
// is not followed) with mode 0600 and writes data.  A partial file is removed.
func writeNew(path string, data []byte) error {
	f, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return err
	}
	_, err = f.Write(data)
	if cerr := f.Close(); err == nil {
		err = cerr
	}
	if err != nil {
		os.Remove(path)
	}
	return err
}

// RunCensus is the command: refuse early, walk, render, write.  A returned error
// is a refusal (nothing was written); otherwise the Exit status says how it went
// and the outputs were written.
func RunCensus(o CensusRunOptions, stdout io.Writer) (*CensusResult, error) {
	for _, p := range []struct{ name, v string }{{"--dir", o.Dir}, {"--out", o.Out}} {
		if !filepath.IsAbs(p.v) {
			return nil, fmt.Errorf("%s %q is not an absolute path", p.name, printable(p.v))
		}
	}
	if o.MD != "" && !filepath.IsAbs(o.MD) {
		return nil, fmt.Errorf("--md %q is not an absolute path", printable(o.MD))
	}
	if o.Records != "" && !filepath.IsAbs(o.Records) {
		return nil, fmt.Errorf("--records %q is not an absolute path", printable(o.Records))
	}
	if o.MD != "" && filepath.Clean(o.MD) == filepath.Clean(o.Out) {
		return nil, errors.New("--out and --md are the same file")
	}
	if err := looksLikeRecordDir(o.Dir); err != nil {
		return nil, err
	}
	// A recording in progress (a step holds the lock shared, collect exclusive)
	// makes the numbers meaningless and its memory use unwelcome: refuse.  The
	// census takes the lock exclusively, without waiting, and holds it while it
	// reads.
	unlock, err := LockDir(o.Dir, true)
	if err != nil {
		return nil, fmt.Errorf("a recording is in progress or the directory cannot be locked: %v", err)
	}
	defer unlock()
	for _, p := range []string{o.Out, o.MD} {
		if p == "" {
			continue
		}
		if err := outsideDir(o.Dir, p); err != nil {
			return nil, err
		}
		if _, err := os.Lstat(p); err == nil {
			return nil, fmt.Errorf("%s already exists", filepath.Base(p))
		}
	}
	acc, err := collectCensus(o.CensusOptions)
	if err != nil {
		return nil, err
	}
	s := acc.summary(CensusTopN)
	res := &CensusResult{Summary: s}
	if o.MD != "" {
		md, top, trunc := RenderCensusMarkdown(s)
		s.Bounds.MDTopPerKind, s.Bounds.MDTruncated = top, trunc
		if err := writeNew(o.MD, []byte(md)); err != nil {
			res.Exit = CensusExitInfra
			return res, fmt.Errorf("writing the Markdown: %v", err)
		}
	}
	data, err := EncodeCensusSummary(s)
	if err == nil {
		err = writeNew(o.Out, data)
	}
	if err != nil {
		res.Exit = CensusExitInfra
		return res, fmt.Errorf("writing the summary: %v", err)
	}
	switch {
	case s.CrossCheck.Mismatches > 0 || s.CrossCheck.Allowed > 0 || s.Problems["parse_panic"] > 0:
		// A parse panic is soak-relevant: a trace that crashes a reader is worse
		// than one the readers disagree on.  Strict, like a mismatch.
		res.Exit = CensusExitMismatch
	case s.InfrastructureFailure != "":
		res.Exit = CensusExitInfra
	}
	cc := s.CrossCheck
	// Counts only: nothing printed here came from a trace.  A sealed shard adds
	// tmpfs_execs, the number that says the exec-head mechanism was exercised.
	tail := ""
	if s.ExecHead.Sealed {
		tail = fmt.Sprintf(" tmpfs_execs=%d", s.ExecHead.TmpfsExecs)
	}
	fmt.Fprintf(stdout, "census: actions=%d traces=%d accepted=%d refused=%d mismatches=%d allowed=%d g2_over_limit_reported=%d problems=%d infra=%v%s restarted_spawns=%d bare_opens=%d\n",
		s.Actions.Total, cc.Traces, cc.Accepted, cc.Refused, cc.Mismatches, cc.Allowed, s.Timing.G2Failing, len(s.Problems), s.InfrastructureFailure != "", tail,
		s.ParseRules.RestartedSpawns, s.ParseRules.BareOpens)
	return res, nil
}

// ExecHeadNoGoLine is the first line of the Exec heads section of a sealed shard
// that exercised the mechanism (tmpfs_execs > 0) but has no Go action.
const ExecHeadNoGoLine = "no Go action in this shard; go_emittable is not a result"

// ExecHeadVacuousLine is the first line of the Exec heads section of a sealed
// shard in which no exec under the step tmpfs reached the exec-head pairing.
const ExecHeadVacuousLine = "exec heads ran but nothing exercised them; go_emittable is not a result"

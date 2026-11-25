//go:build unix

package record

// `kiln record census` (census_report.go, census_render.go): the whole command on
// a real prepared record directory (the step fixture with canned traces), the
// statistics it reports, the hostile names it must carry safely, the untrusted
// files it must survive, and the strict exit statuses.  Unix only: the fixtures
// are POSIX paths, FIFOs and symlinks, so Windows skips them by not building
// them.

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// tracedEntry is a census entry.
type tracedEntry = traced.CensusEntry

// censusDirFix is a prepared shard in which actions A and Z each ran one step
// (a canned helper trace the classifier parses) and the real classify child left
// record.json and census.json beside each.
//
// The canned trace execs /bin/sh, and the classifier then reads the program the
// way the kernel does: it follows symlinks and, for an ELF image, reads its
// loader (PT_INTERP).  What that touches depends on the host.  macOS has a real
// /bin/sh.  A merged-/usr Linux has /bin -> usr/bin, so the path resolves to
// /usr/bin/dash and its loader lives under /lib64 and /usr/lib.  And an "image"
// root compares the ctime of what it covers with the job start, a fixed stamp
// (1794000000) that a runner image built later is newer than: on ubuntu-latest
// the /bin link itself came out "host-read:setup-modified:/bin".  This fixture
// tests the census, not the host-root rules (those have their own tests), so it
// declares every system directory the exec can reach, /bin included, as
// "ephemeral" (covered by nothing, no ctime check); its /bin entry replaces
// prep.commit's default image root.  /sbin stays an image root: the exec never
// reaches it, and prepare refuses a record with no image root at all (the
// sandbox's read-only roots come from them).  A root need not exist (macOS has
// no /lib64).
func censusDirFix(t *testing.T) *stepFix {
	t.Helper()
	lineA, lineZ := playLine("", "a", 0), playLine("", "z", 0)
	roots := []string{
		`{"path": "/bin", "kind": "ephemeral"}`,
		`{"path": "/usr", "kind": "ephemeral"}`,
		`{"path": "/lib", "kind": "ephemeral"}`,
		`{"path": "/lib64", "kind": "ephemeral"}`,
	}
	f := newStepFixRoots(t, roots, planOf([]string{lineA}, nil, []string{lineZ}), nil, nil, true)
	f.give("a", actA, lineA, 0, cannedOpts{})
	f.give("z", actZ, lineZ, 0, cannedOpts{})
	f.must(actA, 0, 1, lineA)
	f.must(actZ, 0, 1, lineZ)
	return f
}

// runCensus runs the command over f's record directory, with its outputs in a
// fresh directory, and returns the result and the parsed summary.
func runCensus(t *testing.T, f *stepFix, mutate ...func(*CensusRunOptions)) (*CensusResult, *CensusSummary, string, string) {
	t.Helper()
	out := t.TempDir()
	o := CensusRunOptions{CensusOptions: CensusOptions{Dir: f.p.dir}, Out: filepath.Join(out, "census.json"), MD: filepath.Join(out, "census.md")}
	for _, m := range mutate {
		m(&o)
	}
	var stdout bytes.Buffer
	res, err := RunCensus(o, &stdout)
	if err != nil {
		t.Fatalf("RunCensus: %v", err)
	}
	return res, loadSummary(t, o.Out), o.Out, o.MD
}

// loadSummary decodes a summary file strictly: an unknown key is a failure.
func loadSummary(t *testing.T, path string) *CensusSummary {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.DisallowUnknownFields()
	var s CensusSummary
	if err := dec.Decode(&s); err != nil {
		t.Fatalf("the summary does not decode: %v", err)
	}
	if s.Schema != CensusSummarySchema {
		t.Fatalf("schema %q", s.Schema)
	}
	return &s
}

// putCensus replaces action's census.json with data.
func putCensus(t *testing.T, f *stepFix, action string, data []byte) {
	t.Helper()
	p := CensusFile(f.p.dir, action)
	os.RemoveAll(p)
	writeBytes(t, p, data, 0o644)
}

func putCensusDoc(t *testing.T, f *stepFix, action string, doc CensusDoc) {
	t.Helper()
	if doc.Schema == "" {
		doc.Schema = CensusSchema
	}
	if doc.Action == "" {
		doc.Action = action
	}
	if doc.Overflow == nil {
		doc.Overflow = map[string]int64{}
	}
	data, err := json.Marshal(doc)
	if err != nil {
		t.Fatal(err)
	}
	putCensus(t, f, action, data)
}

// The whole command on a clean recording: both actions parsed and cross-checked,
// every statistic present, both files written (0600), exit 0.
func TestCensusEndToEnd(t *testing.T) {
	f := censusDirFix(t)
	res, s, jsonPath, mdPath := runCensus(t, f)
	if res.Exit != 0 || s.InfrastructureFailure != "" {
		t.Fatalf("exit %d, infra %q, problems %v", res.Exit, s.InfrastructureFailure, s.Problems)
	}
	if len(s.Problems) != 0 {
		t.Errorf("problems: %v", s.Problems)
	}
	a := s.Actions
	if a.Total != 2 || a.WithRecord != 2 || a.RecordParseOK != 2 || a.CensusClean != 2 || a.NoVerdict != 0 || a.CensusMissing != 0 {
		// The whole summary, so a host-dependent census (one OS classifies the
		// canned exec differently) says WHAT it found, not only that it is
		// non-empty.  Bounded: the summary is at most 512 KiB, cut to 8 KiB.
		summary, _ := os.ReadFile(jsonPath)
		if len(summary) > 8<<10 {
			summary = summary[:8<<10]
		}
		t.Errorf("actions: %+v\nsummary: %s", a, summary)
	}
	cc := s.CrossCheck
	if cc.Traces != 2 || cc.Accepted != 2 || cc.Mismatches != 0 || cc.Allowed != 0 || cc.Refused != 0 || cc.Bytes <= 0 {
		t.Errorf("cross-check: %+v", cc)
	}
	if s.TraceSizes.All.N != 2 || s.TraceSizes.RootOK.N != 2 || s.TraceSizes.Lines.N != 2 || s.TraceSizes.Lines.Max < 10 || len(s.TraceSizes.All.Buckets) == 0 {
		t.Errorf("sizes: %+v", s.TraceSizes)
	}
	tm := s.Timing
	if tm.ParseMS.N != 2 || tm.StepWallMS.N != 2 || tm.ParseShare.N != 2 || tm.WallsDropped != 0 || tm.ParseRefused != 0 {
		t.Errorf("timing: %+v", tm)
	}
	if s.PerAction.Bytes.N != 2 || s.PerAction.CapBytes != DefaultMaxTraceTotal || len(s.PerAction.Largest) != 2 || s.PerAction.OverCap != 0 {
		t.Errorf("per action: %+v", s.PerAction)
	}
	if len(s.Languages) != 1 || s.Languages[0].Actions != 2 || s.Languages[0].Recordable != 2 || s.Languages[0].Clean != 2 || s.Languages[0].Share != 1 {
		t.Errorf("languages: %+v", s.Languages)
	}
	for _, p := range []string{jsonPath, mdPath} {
		st, err := os.Stat(p)
		if err != nil || st.Mode().Perm() != 0o600 {
			t.Errorf("%s: %v %v", p, st, err)
		}
	}
	md, _ := os.ReadFile(mdPath)
	for _, want := range []string{"# Kiln record census", "cross-check: ok", "## Reasons by kind", "## Share by language"} {
		if !strings.Contains(string(md), want) {
			t.Errorf("the Markdown lacks %q", want)
		}
	}
	// It never writes inside the record directory.
	for _, d := range []string{f.p.dir, filepath.Join(f.p.dir, "traces", filepath.Base(StepDir(f.p.dir, actA)))} {
		ents, _ := os.ReadDir(d)
		for _, e := range ents {
			if strings.Contains(e.Name(), "census.md") || e.Name() == "census.json" && d == f.p.dir {
				t.Errorf("a census output appeared in the record directory: %s", e.Name())
			}
		}
	}
}

// Histograms: by kind, by prefix (depth at most 4) and the top full paths, with
// counts summed across actions and the classifier's own overflow carried.
func TestCensusHistograms(t *testing.T) {
	f := censusDirFix(t)
	putCensusDoc(t, f, actZ, CensusDoc{Total: 70, OverflowTotal: 5, Overflow: map[string]int64{"host-read": 5}, Entries: []tracedEntry{
		{Kind: "host-read", Path: "/usr/lib/x86_64-linux-gnu/gconv/gconv-modules", Count: 30},
		{Kind: "host-read", Path: "/usr/lib/x86_64-linux-gnu/gconv/UTF-16.so", Count: 10},
		{Kind: "host-read", Path: "/etc/passwd", Count: 20},
		{Kind: "trace-incomplete:proc-path", Path: "/proc/9/cmdline", Count: 5},
	}})
	putCensusDoc(t, f, actA, CensusDoc{Total: 7, Entries: []tracedEntry{{Kind: "host-read", Path: "/etc/passwd", Count: 7}}})
	res, s, _, _ := runCensus(t, f)
	if res.Exit != 0 {
		t.Fatalf("exit %d", res.Exit)
	}
	if s.Actions.CensusNonEmpty != 2 || s.Actions.CensusClean != 0 {
		t.Errorf("actions: %+v", s.Actions)
	}
	if len(s.Reasons) != 2 || s.Reasons[0].Kind != "host-read" || s.Reasons[1].Kind != "trace-incomplete:proc-path" {
		t.Fatalf("kinds: %+v", s.Reasons)
	}
	h := s.Reasons[0]
	if h.Occurrences != 67 || h.Overflow != 5 || h.Actions != 2 || h.DistinctPaths != 3 {
		t.Errorf("host-read: %+v", h)
	}
	if h.TopPaths[0] != (PathRow{"/usr/lib/x86_64-linux-gnu/gconv/gconv-modules", 30}) || h.TopPaths[1] != (PathRow{"/etc/passwd", 27}) {
		t.Errorf("top paths: %+v", h.TopPaths)
	}
	got := map[string]PrefixRow{}
	for _, p := range h.Prefixes {
		got[p.Prefix] = p
	}
	if p := got["/usr/lib/x86_64-linux-gnu/gconv"]; p.Paths != 2 || p.Count != 40 {
		t.Errorf("prefixes: %+v", h.Prefixes)
	}
	if p := got["/etc/passwd"]; p.Paths != 1 || p.Count != 27 {
		t.Errorf("prefixes: %+v", h.Prefixes)
	}
	if s.Reasons[1].Occurrences != 5 || s.Reasons[1].Actions != 1 {
		t.Errorf("proc-path: %+v", s.Reasons[1])
	}
}

// Redaction: the home directory, the temporary directory and the record
// directory are labels, never the paths.
func TestCensusRedactsDirectories(t *testing.T) {
	f := censusDirFix(t)
	putCensusDoc(t, f, actZ, CensusDoc{Total: 3, Entries: []tracedEntry{
		{Kind: "host-read", Path: "/home/runner/.cache/go/x", Count: 1},
		{Kind: "host-read", Path: f.p.dir + "/pre/SEALED", Count: 1},
		{Kind: "host-read", Path: "/var/tmp/job7/out", Count: 1},
	}})
	_, s, jsonPath, mdPath := runCensus(t, f, func(o *CensusRunOptions) {
		o.Redact = []Redaction{{From: "/home/runner", To: "$HOME"}, {From: "/var/tmp/job7", To: "$TMPDIR"}}
	})
	var paths []string
	for _, p := range s.Reasons[0].TopPaths {
		paths = append(paths, p.Path)
	}
	joined := strings.Join(paths, "\n")
	for _, want := range []string{"$RUNNER_HOME/.cache/go/x", "$RECORD_DIR/pre/SEALED", "$RUNNER_TMP/out"} {
		if !strings.Contains(joined, want) {
			t.Errorf("no %q in %v", want, paths)
		}
	}
	for _, p := range []string{jsonPath, mdPath} {
		data, _ := os.ReadFile(p)
		for _, secret := range []string{"/home/runner", "/var/tmp/job7", f.p.dir} {
			if bytes.Contains(data, []byte(secret)) {
				t.Errorf("%s leaks %q", filepath.Base(p), secret)
			}
		}
	}
}

// Hostile names: a newline, a workflow command, backtick runs, a pipe, markup, a
// bidi override, a 5000-character path and 10,000 distinct paths all come out of
// BOTH files safe.
func TestCensusHostileNames(t *testing.T) {
	f := censusDirFix(t)
	hostile := []string{
		"/tmp/a\nb", "::set-output name=x::1", "/x/`y`", "/x/```|z|```", "/x/<script>alert(1)</script>",
		"/x/‮gnp.exe", "/x/a|b|c", "/" + strings.Repeat("A", 5000), "\n::error::forged", "/x/[l](http://evil) *b* _i_ # h &amp;",
	}
	var entries []tracedEntry
	for i, p := range hostile {
		entries = append(entries, tracedEntry{Kind: "host-read", Path: p, Count: int64(10 + i)})
	}
	for i := 0; i < 10000; i++ {
		entries = append(entries, tracedEntry{Kind: "network", Path: fmt.Sprintf("/d/%05d", i), Count: 1})
	}
	putCensusDoc(t, f, actZ, CensusDoc{Total: 9999, Entries: entries})
	res, s, jsonPath, mdPath := runCensus(t, f)
	if res.Exit != 0 {
		t.Fatalf("exit %d, %v", res.Exit, s.Problems)
	}
	if len(s.Problems) != 0 {
		t.Fatalf("the census was refused: %v", s.Problems)
	}
	var net, host *KindSummary
	for i := range s.Reasons {
		switch s.Reasons[i].Kind {
		case "network":
			net = &s.Reasons[i]
		case "host-read":
			host = &s.Reasons[i]
		}
	}
	if net == nil || host == nil || net.DistinctPaths != 10000 || len(net.TopPaths) != CensusTopN || host.DistinctPaths != len(hostile) {
		t.Fatalf("kinds: %+v", s.Reasons)
	}
	if s.Bounds.PathsTruncated != 1 {
		t.Errorf("paths truncated = %d, want 1", s.Bounds.PathsTruncated)
	}
	var long string
	have := map[string]bool{}
	for _, p := range host.TopPaths {
		have[p.Path] = true
		if utf8.RuneCountInString(p.Path) > 515 {
			t.Errorf("a path of %d runes", utf8.RuneCountInString(p.Path))
		}
		if strings.HasSuffix(p.Path, "...") && strings.HasPrefix(p.Path, "/AAAA") {
			long = p.Path
		}
	}
	if long == "" || !have["/tmp/a?b"] || !have["/x/?gnp.exe"] || !have["/x/`y`"] {
		t.Errorf("the sanitised paths are not in the summary: %v", have)
	}

	jdata, _ := os.ReadFile(jsonPath)
	if len(jdata) > MaxSummaryJSONBytes || bytes.Count(jdata, []byte("\n")) != 1 || bytes.Contains(jdata, []byte("‮")) {
		t.Errorf("the JSON: %d bytes, %d newlines", len(jdata), bytes.Count(jdata, []byte("\n")))
	}
	if bytes.Contains(jdata, []byte("<script>")) { // encoding/json escapes it
		t.Error("the JSON holds a raw <script>")
	}

	mdata, _ := os.ReadFile(mdPath)
	md := string(mdata)
	if len(md) > MaxSummaryMDBytes || !utf8.ValidString(md) {
		t.Fatalf("the Markdown: %d bytes", len(md))
	}
	for i, l := range strings.Split(md, "\n") {
		if strings.HasPrefix(l, "::") {
			t.Errorf("line %d starts with ::", i+1)
		}
		for _, r := range l {
			if r < 0x20 || r == 0x7f || r == 0x202e || r == 0x2028 {
				t.Errorf("line %d holds the control character %U", i+1, r)
			}
		}
		if strings.Contains(l, "set-output") && !strings.HasPrefix(l, "- ") {
			t.Errorf("line %d: attacker text outside a list item: %q", i+1, l)
		}
	}
	// Outside its code span, none of the attacker's text appears at all.
	outside := stripSpans(md)
	for _, bad := range []string{"set-output", "<script", "forged", "evil", "|z|", "AAAA", "gnp"} {
		if strings.Contains(outside, bad) {
			t.Errorf("%q occurs outside a code span", bad)
		}
	}
	// And no table row holds a path.
	for _, l := range strings.Split(md, "\n") {
		if strings.HasPrefix(l, "|") && strings.ContainsAny(l, "/<`") {
			t.Errorf("a table row holds attacker-shaped text: %q", l)
		}
	}
}

// Everything the child left is read as hostile.  Each bad census.json is counted
// under its own problem code and does not stop the other action, and none can
// hang or crash the command.
func TestCensusUntrustedInputs(t *testing.T) {
	valid := func(action string) map[string]any {
		return map[string]any{"schema": CensusSchema, "action": action, "total": 1, "overflow_total": 0, "overflow": map[string]int64{},
			"entries": []map[string]any{{"kind": "host-read", "path": "/x", "count": 1}}}
	}
	enc := func(m map[string]any) []byte { b, _ := json.Marshal(m); return b }
	with := func(k string, v any) []byte { m := valid(actA); m[k] = v; return enc(m) }
	for _, c := range []struct {
		name string
		code string
		put  func(t *testing.T, f *stepFix, p string)
	}{
		{"a symlink", "census_unreadable", func(t *testing.T, f *stepFix, p string) {
			target := filepath.Join(t.TempDir(), "elsewhere.json")
			writeBytes(t, target, enc(valid(actA)), 0o644)
			if err := os.Symlink(target, p); err != nil {
				t.Fatal(err)
			}
		}},
		{"a dangling symlink", "census_unreadable", func(t *testing.T, f *stepFix, p string) {
			if err := os.Symlink("/nonexistent/x", p); err != nil {
				t.Fatal(err)
			}
		}},
		{"a FIFO", "census_unreadable", func(t *testing.T, f *stepFix, p string) {
			if err := syscall.Mkfifo(p, 0o600); err != nil {
				t.Skipf("no FIFOs here: %v", err)
			}
		}},
		{"a directory", "census_unreadable", func(t *testing.T, f *stepFix, p string) {
			if err := os.Mkdir(p, 0o755); err != nil {
				t.Fatal(err)
			}
		}},
		{"an oversized file", "census_unreadable", func(t *testing.T, f *stepFix, p string) {
			writeBytes(t, p, []byte(`{"schema":"`+strings.Repeat("a", MaxCensusBytes)+`"}`), 0o644)
		}},
		{"an unknown field", "census_decode", func(t *testing.T, f *stepFix, p string) { writeBytes(t, p, with("extra", 1), 0o644) }},
		{"a duplicate key", "census_decode", func(t *testing.T, f *stepFix, p string) {
			writeBytes(t, p, []byte(`{"schema":"`+CensusSchema+`","schema":"`+CensusSchema+`","action":"`+actA+`","total":0,"overflow_total":0,"overflow":{},"entries":[]}`), 0o644)
		}},
		{"trailing data", "census_decode", func(t *testing.T, f *stepFix, p string) {
			writeBytes(t, p, append(enc(valid(actA)), []byte(` {}`)...), 0o644)
		}},
		{"not JSON", "census_decode", func(t *testing.T, f *stepFix, p string) { writeBytes(t, p, []byte("\x00\xff garbage"), 0o644) }},
		{"a wrong schema", "census_schema", func(t *testing.T, f *stepFix, p string) {
			writeBytes(t, p, with("schema", "kiln-record-census/2"), 0o644)
		}},
		{"another action", "census_action_mismatch", func(t *testing.T, f *stepFix, p string) { writeBytes(t, p, with("action", actZ), 0o644) }},
		{"a negative total", "census_bounds", func(t *testing.T, f *stepFix, p string) { writeBytes(t, p, with("total", -1), 0o644) }},
		{"a negative count", "census_bad_entry", func(t *testing.T, f *stepFix, p string) {
			writeBytes(t, p, with("entries", []map[string]any{{"kind": "host-read", "path": "/x", "count": -4}}), 0o644)
		}},
		{"a bad kind", "census_bad_entry", func(t *testing.T, f *stepFix, p string) {
			writeBytes(t, p, with("entries", []map[string]any{{"kind": "Host Read\n::x", "path": "/x", "count": 1}}), 0o644)
		}},
		{"a bad overflow kind", "census_bad_entry", func(t *testing.T, f *stepFix, p string) {
			writeBytes(t, p, with("overflow", map[string]int64{"a b": 1}), 0o644)
		}},
	} {
		t.Run(c.name, func(t *testing.T) {
			f := censusDirFix(t)
			p := CensusFile(f.p.dir, actA)
			os.RemoveAll(p)
			c.put(t, f, p)
			done := make(chan struct{})
			var res *CensusResult
			var s *CensusSummary
			go func() {
				defer close(done)
				res, s, _, _ = runCensus(t, f)
			}()
			select {
			case <-done:
			case <-time.After(30 * time.Second):
				t.Fatal("the command hung")
			}
			if res.Exit != 0 {
				t.Errorf("exit %d", res.Exit)
			}
			if s.Actions.CensusInvalid != 1 || s.Actions.CensusClean != 1 || s.Problems[c.code] != 1 {
				t.Errorf("actions %+v, problems %v, want one %s", s.Actions, s.Problems, c.code)
			}
			if s.CrossCheck.Traces != 2 {
				t.Errorf("the other action's trace was not checked: %+v", s.CrossCheck)
			}
		})
	}
}

// The states that are NOT "no host reads": a missing census, an empty census
// beside a failed parse half, an empty one with no record, and a record.failed.
func TestCensusEmptyAndMissingAreCountedSeparately(t *testing.T) {
	f := censusDirFix(t)
	os.Remove(CensusFile(f.p.dir, actA)) // classify error: no census.json
	// Z: a failed parse half: record.json says parse_ok false, and the census is empty.
	bad, err := EncodeClassified(ClassifiedRecord{Schema: ClassifiedSchema, Action: actZ, BindingID: f.pre.BindingID,
		SealedSHA256: f.pre.SealedSHA256, StepsRun: 1, ParseReasons: []string{"step 0: the trace cannot be opened"}})
	if err != nil {
		t.Fatal(err)
	}
	os.Remove(RecordFile(f.p.dir, actZ))
	writeBytes(t, RecordFile(f.p.dir, actZ), bad, 0o644)
	res, s, _, _ := runCensus(t, f)
	a := s.Actions
	if res.Exit != 0 || a.Total != 2 || a.CensusMissing != 1 || a.CensusEmptyParseFailed != 1 || a.CensusClean != 0 || a.CensusNonEmpty != 0 {
		t.Errorf("exit %d actions %+v", res.Exit, a)
	}
	if a.WithRecord != 2 || a.RecordParseOK != 1 {
		t.Errorf("records: %+v", a)
	}
	// A is recordable but its census is missing, so it is not "clean".
	if l := s.Languages[0]; l.Recordable != 1 || l.Clean != 0 {
		t.Errorf("language: %+v", l)
	}

	// record.failed, and no record at all.
	f = censusDirFix(t)
	os.Remove(RecordFile(f.p.dir, actA))
	writeBytes(t, RecordFailedFile(f.p.dir, actA), []byte("timeout: the classifier ran too long"), 0o644)
	os.Remove(RecordFile(f.p.dir, actZ))
	_, s, _, _ = runCensus(t, f)
	a = s.Actions
	if a.RecordFailed != 1 || a.NoVerdict != 1 || a.WithRecord != 0 || a.CensusEmptyNoRecord != 2 {
		t.Errorf("actions: %+v", a)
	}
	// The reason text of record.failed is never read into the report.
	if data, _ := json.Marshal(s); bytes.Contains(data, []byte("classifier ran too long")) {
		t.Error("record.failed's text reached the summary")
	}
}

// A wall that is not positive is dropped and counted, never divided by.
func TestCensusDropsNonPositiveWalls(t *testing.T) {
	f := censusDirFix(t)
	sc := f.sidecar(actA, 0)
	sc.StartedAt, sc.EndedAt = sc.EndedAt, sc.StartedAt
	if sc.StartedAt == sc.EndedAt {
		t.Skip("the two stamps are equal")
	}
	data, err := EncodeStepSidecar(sc)
	if err != nil {
		t.Fatal(err)
	}
	p := StepFile(f.p.dir, actA, 0, "json")
	os.Remove(p)
	writeBytes(t, p, data, 0o644)
	_, s, _, _ := runCensus(t, f)
	if s.Timing.WallsDropped != 1 || s.Timing.StepWallMS.N != 1 || s.Timing.ParseMS.N != 2 || s.Timing.ParseShare.N != 1 {
		t.Errorf("timing: %+v", s.Timing)
	}
}

// The cross-check is strict: a mismatch, or any allowed divergence, is exit 4,
// and the summary still names it (by digest prefix, never by trace text).
func TestCensusStrictCrossCheck(t *testing.T) {
	type parse = func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, trace.CrossCheckDetail)
	for name, stub := range map[string]parse{
		"a mismatch": func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, trace.CrossCheckDetail) {
			return nil, nil, &trace.CrossCheckMismatch{Detail: "FAIL-OPEN: x", Window: "  > 1: \"secret trace line\""}, trace.CrossCheckDetail{}
		},
		"an allowed divergence": func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, trace.CrossCheckDetail) {
			return nil, nil, nil, trace.CrossCheckDetail{Allowed: 1, Rows: map[string]int{"signed pid": 1}, Hits: []trace.AllowedHit{{Row: "signed pid", Line: 1}}}
		},
	} {
		t.Run(name, func(t *testing.T) {
			f := censusDirFix(t)
			res, s, _, mdPath := runCensus(t, f, func(o *CensusRunOptions) { o.CrossCheck = stub })
			if res.Exit != CensusExitMismatch {
				t.Fatalf("exit %d, want %d", res.Exit, CensusExitMismatch)
			}
			cc := s.CrossCheck
			if cc.Mismatches+cc.Allowed == 0 || len(cc.Details) == 0 {
				t.Errorf("cross-check: %+v", cc)
			}
			data, _ := json.Marshal(s)
			md, _ := os.ReadFile(mdPath)
			if bytes.Contains(data, []byte("secret trace line")) || bytes.Contains(md, []byte("secret trace line")) {
				t.Error("a window of trace lines reached the report")
			}
			if !strings.Contains(string(md), "cross-check: FAILED") {
				t.Error("the Markdown does not say FAILED")
			}
		})
	}
}

// No traces is an infrastructure failure (exit 1), still reported; an unverifiable
// pre/ is one too, and nothing is parsed.
func TestCensusInfrastructureFailures(t *testing.T) {
	f := newStepFixCustom(t, planOf(nil, nil, nil), nil, true)
	if err := os.MkdirAll(filepath.Join(f.p.dir, "traces"), 0o755); err != nil {
		t.Fatal(err)
	}
	res, s, _, _ := runCensus(t, f)
	if res.Exit != CensusExitInfra || !strings.Contains(s.InfrastructureFailure, "no step trace") || s.CrossCheck.Traces != 0 {
		t.Errorf("no traces: exit %d, %q", res.Exit, s.InfrastructureFailure)
	}

	f = censusDirFix(t)
	if err := os.Chmod(filepath.Join(f.p.dir, "pre"), 0o700); err != nil {
		t.Fatal(err)
	}
	os.Remove(filepath.Join(f.p.dir, "pre", "SEALED"))
	res, s, _, _ = runCensus(t, f)
	if res.Exit != CensusExitInfra || !strings.Contains(s.InfrastructureFailure, "pre/ does not verify") || s.CrossCheck.Traces != 0 || s.Problems["pre_unverified"] != 1 {
		t.Errorf("a broken pre/: exit %d, %q, %v", res.Exit, s.InfrastructureFailure, s.Problems)
	}
	// The histograms and sizes still come out.
	if s.TraceSizes.All.N != 2 {
		t.Errorf("sizes without pre/: %+v", s.TraceSizes)
	}
	if strings.Contains(s.InfrastructureFailure, f.p.dir) {
		t.Error("the record directory path is not redacted")
	}
}

// Refusals write nothing.
func TestCensusRefusals(t *testing.T) {
	f := censusDirFix(t)
	out := t.TempDir()
	good := filepath.Join(out, "ok.json")
	existing := filepath.Join(out, "there.json")
	writeBytes(t, existing, []byte("keep"), 0o644)
	linkIn := filepath.Join(out, "link")
	if err := os.Symlink(f.p.dir, linkIn); err != nil {
		t.Fatal(err)
	}
	for name, o := range map[string]CensusRunOptions{
		"relative dir":       {CensusOptions: CensusOptions{Dir: "rec"}, Out: good},
		"relative out":       {CensusOptions: CensusOptions{Dir: f.p.dir}, Out: "out.json"},
		"relative md":        {CensusOptions: CensusOptions{Dir: f.p.dir}, Out: good, MD: "x.md"},
		"relative records":   {CensusOptions: CensusOptions{Dir: f.p.dir, Records: "r.jsonl"}, Out: good},
		"not a record dir":   {CensusOptions: CensusOptions{Dir: out}, Out: good},
		"out exists":         {CensusOptions: CensusOptions{Dir: f.p.dir}, Out: existing},
		"md exists":          {CensusOptions: CensusOptions{Dir: f.p.dir}, Out: good, MD: existing},
		"out inside dir":     {CensusOptions: CensusOptions{Dir: f.p.dir}, Out: filepath.Join(f.p.dir, "census.json")},
		"md inside traces":   {CensusOptions: CensusOptions{Dir: f.p.dir}, Out: good, MD: filepath.Join(f.p.dir, "traces", "x.md")},
		"out through a link": {CensusOptions: CensusOptions{Dir: f.p.dir}, Out: filepath.Join(linkIn, "traces", "o.json")},
		"same file":          {CensusOptions: CensusOptions{Dir: f.p.dir}, Out: good, MD: good},
		"no parent":          {CensusOptions: CensusOptions{Dir: f.p.dir}, Out: filepath.Join(out, "nope", "o.json")},
	} {
		if _, err := RunCensus(o, io.Discard); err == nil {
			t.Errorf("%s: no refusal", name)
		}
	}
	if _, err := os.Stat(good); err == nil {
		t.Error("a refusal wrote an output")
	}
	if data, _ := os.ReadFile(existing); string(data) != "keep" {
		t.Error("an existing output was overwritten")
	}
	for _, p := range []string{filepath.Join(f.p.dir, "census.json"), filepath.Join(f.p.dir, "traces", "x.md")} {
		if exists(p) {
			t.Errorf("a refusal wrote %s", p)
		}
	}
}

// With --records, collect's header and the record count join the report.
func TestCensusReadsCollectHeader(t *testing.T) {
	f := censusDirFix(t)
	records := filepath.Join(t.TempDir(), "records.jsonl")
	hdr := fmt.Sprintf(`{"schema":"kiln-record-shard/1","binding_id":"%s","shard_index":2,"shard_count":3,"void":["survivor","tracked-changed"],"post_inventory_unexplained":4,"tracked_changed_unexplained":1}`, f.pre.BindingID)
	writeBytes(t, records, []byte(hdr+"\n{\"a\":1}\n{\"a\":2}\n{\"a\":3}\n"), 0o644)
	_, s, _, mdPath := runCensus(t, f, func(o *CensusRunOptions) { o.Records = records })
	c := s.Collect
	if c == nil {
		t.Fatalf("no collect section: %v", s.Problems)
	}
	if c.ShardIndex != 2 || c.ShardCount != 3 || c.Records != 3 || c.PostInventoryUnexplained != 4 || c.TrackedChangedUnexplained != 1 || len(c.Void) != 2 || c.Void[1] != "tracked-changed" {
		t.Errorf("collect: %+v", c)
	}
	md, _ := os.ReadFile(mdPath)
	if !strings.Contains(string(md), "## collect") {
		t.Error("no collect section in the Markdown")
	}
	// An unreadable or malformed records file is a counted problem, not a failure.
	for name, put := range map[string]func(p string){
		"a symlink": func(p string) { os.Symlink(records, p) },
		"garbage":   func(p string) { os.WriteFile(p, []byte("not a header\n"), 0o644) },
		"no newline": func(p string) {
			os.WriteFile(p, []byte(hdr), 0o644)
		},
	} {
		bad := filepath.Join(t.TempDir(), "r.jsonl")
		put(bad)
		res, s, _, _ := runCensus(t, f, func(o *CensusRunOptions) { o.Records = bad })
		if res.Exit != 0 || s.Collect != nil || len(s.Problems) != 1 {
			t.Errorf("%s: exit %d collect %v problems %v", name, res.Exit, s.Collect, s.Problems)
		}
	}
}

// A stray directory, a symlinked action directory, and a trace without a
// sidecar are counted and skipped.
func TestCensusStrayEntries(t *testing.T) {
	f := censusDirFix(t)
	traces := filepath.Join(f.p.dir, "traces")
	os.Mkdir(filepath.Join(traces, "not-a-digest"), 0o755)
	os.WriteFile(filepath.Join(traces, ".seq"), []byte("2"), 0o644)
	os.Symlink(StepDir(f.p.dir, actA), filepath.Join(traces, strings.Repeat("a", 64)))
	writeBytes(t, filepath.Join(StepDir(f.p.dir, actA), "7.trace"), []byte("1  exit_group(0) = ?\n"), 0o644)
	_, s, _, _ := runCensus(t, f)
	if s.Problems["stray_traces_entry"] != 2 || s.Problems["trace_without_sidecar"] != 1 || s.Actions.Total != 2 {
		t.Errorf("problems %v actions %+v", s.Problems, s.Actions)
	}
}

// The recorder's own marker files under traces/ (including the state-preloaded
// marker that collect writes into every runner-home shard) are not stray; a
// genuinely stray file still is.
func TestCensusStateMarkerNotStray(t *testing.T) {
	f := censusDirFix(t)
	traces := filepath.Join(f.p.dir, "traces")
	for _, n := range []string{StatePreloadedMarker, CollectedMarker, SerialFile} {
		if err := os.WriteFile(filepath.Join(traces, n), []byte("x"), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	_, s, _, _ := runCensus(t, f)
	if s.Problems["stray_traces_entry"] != 0 {
		t.Errorf("markers counted as stray: %v", s.Problems)
	}
	if err := os.WriteFile(filepath.Join(traces, "stray.txt"), []byte("x"), 0o600); err != nil {
		t.Fatal(err)
	}
	_, s, _, _ = runCensus(t, f)
	if s.Problems["stray_traces_entry"] != 1 {
		t.Errorf("stray file not counted once: %v", s.Problems)
	}
}

// A marker name is only a marker as a regular file: a directory, or a link to
// one, with that name is stray, and is not reported as the marker's presence.
func TestCensusMarkerNameAsDirectoryIsStray(t *testing.T) {
	for name, mk := range map[string]func(p, other string) error{
		"directory": func(p, _ string) error { return os.Mkdir(p, 0o755) },
		"dir link":  func(p, other string) error { return os.Symlink(other, p) },
	} {
		f := censusDirFix(t)
		traces := filepath.Join(f.p.dir, "traces")
		if err := mk(filepath.Join(traces, StatePreloadedMarker), t.TempDir()); err != nil {
			t.Fatal(err)
		}
		_, s, _, _ := runCensus(t, f)
		if s.Problems["stray_traces_entry"] != 1 || s.StatePreloadedMarker {
			t.Errorf("%s: problems %v marker %v", name, s.Problems, s.StatePreloadedMarker)
		}
	}
}

// The marker against collect's void list. Matching void: no problem. Marker
// without the void: the named problem. No collect block: the fact only.
func TestCensusStatePreloadedMarkerConsistency(t *testing.T) {
	for name, tc := range map[string]struct {
		void        string // the header's void list as JSON, "" for no records file
		wantProblem int
	}{
		"matching void":     {`["state-preloaded"]`, 0},
		"marker, no void":   {`["survivor"]`, 1},
		"no collect block":  {"", 0},
		"empty void":        {`[]`, 1},
		"other tokens plus": {`["survivor","state-preloaded"]`, 0},
	} {
		f := censusDirFix(t)
		if err := os.WriteFile(filepath.Join(f.p.dir, "traces", StatePreloadedMarker), []byte("/state"), 0o600); err != nil {
			t.Fatal(err)
		}
		records := ""
		if tc.void != "" {
			records = filepath.Join(t.TempDir(), "records.jsonl")
			hdr := fmt.Sprintf(`{"schema":"kiln-record-shard/1","binding_id":"%s","shard_index":0,"shard_count":1,"void":%s,"post_inventory_unexplained":0,"tracked_changed_unexplained":0}`, f.pre.BindingID, tc.void)
			writeBytes(t, records, []byte(hdr+"\n"), 0o644)
		}
		_, s, _, mdPath := runCensus(t, f, func(o *CensusRunOptions) { o.Records = records })
		if got := s.Problems["state_preloaded_marker_without_void"]; got != tc.wantProblem {
			t.Errorf("%s: problem count %d, want %d (%v)", name, got, tc.wantProblem, s.Problems)
		}
		if s.Problems["stray_traces_entry"] != 0 || !s.StatePreloadedMarker {
			t.Errorf("%s: stray %d marker fact %v", name, s.Problems["stray_traces_entry"], s.StatePreloadedMarker)
		}
		md, err := os.ReadFile(mdPath)
		if err != nil || !strings.Contains(string(md), "state-preloaded marker present: true") {
			t.Errorf("%s: Markdown lacks the marker fact (%v)", name, err)
		}
	}
}

// ---------------------------------------------------------------------------
// Synthetic summaries (no record directory)
// ---------------------------------------------------------------------------

func TestCensusLanguageFromPath(t *testing.T) {
	r := &censusRun{}
	for in, want := range map[string]string{
		"code/packages/go/foo": "go", "code/programs/rust/bar": "rust", "code/packages/C++/x": "other",
		"foo": "other", "code/packages/": "other", "code/other/go/x": "other", "code/packages/" + strings.Repeat("a", 40) + "/x": "other",
	} {
		if got := r.language(in); got != want {
			t.Errorf("language(%q) = %q, want %q", in, got, want)
		}
	}
}

// The per-action cap counters, the G2 counters and the language share, from a
// hand-built accumulator.
func TestCensusSummaryStatistics(t *testing.T) {
	a := newCensusAcc(nil)
	gib := int64(1 << 30)
	a.actionBytes = []namedBytes{{"a", gib}, {"b", gib * 19 / 10}, {"c", 2 * gib}, {"d", 3 * gib}, {"e", 10}}
	a.walls, a.parses, a.shares = []float64{100, 200}, []float64{5, 6000}, []float64{0.05, 0.3}
	a.over10, a.over5s, a.g2, a.wallsDropped, a.parseRefused = 1, 1, 1, 2, 3
	a.langs["go"] = &langAcc{actions: 4, recordable: 3, clean: 1}
	a.langs["other"] = &langAcc{}
	s := a.summary(CensusTopN)
	pa := s.PerAction
	if pa.Over50 != 4 || pa.Over90 != 3 || pa.OverCap != 2 || pa.Bytes.N != 5 || pa.Largest[0].Action != "d" || pa.Largest[0].PercentCap != 150 {
		t.Errorf("per action: %+v", pa)
	}
	tm := s.Timing
	if tm.ShareOver10Pct != 1 || tm.ParseOver5s != 1 || tm.G2Failing != 1 || tm.WallsDropped != 2 || tm.ParseRefused != 3 || tm.ParseMS.Max != 6000 || tm.ParseShare.P50 != 0.05 {
		t.Errorf("timing: %+v", tm)
	}
	if l := s.Languages[0]; l.Language != "go" || l.Share != 0.75 || s.Languages[1].Share != 0 {
		t.Errorf("languages: %+v", s.Languages)
	}
}

// Both outputs stay within their bounds, by cutting the lists and saying so.
func TestCensusBoundsCutLists(t *testing.T) {
	a := newCensusAcc(nil)
	for k := 0; k < 8; k++ {
		var doc CensusDoc
		for i := 0; i < 300; i++ {
			doc.Entries = append(doc.Entries, tracedEntry{Kind: fmt.Sprintf("kind-%d", k),
				Path: fmt.Sprintf("/%c%d/", 'a'+i%26, i) + strings.Repeat("x", 600), Count: int64(i + 1)})
		}
		a.addCensus(&doc)
	}
	s := a.summary(CensusTopN)
	if s.Bounds.PathsTruncated != 2400 {
		t.Errorf("truncated %d", s.Bounds.PathsTruncated)
	}
	data, err := EncodeCensusSummary(s)
	if err != nil || len(data) > MaxSummaryJSONBytes {
		t.Fatalf("%d bytes, %v", len(data), err)
	}
	var got CensusSummary
	if err := json.Unmarshal(data, &got); err != nil {
		t.Fatal(err)
	}
	if !got.Bounds.JSONListsCut || got.Bounds.TopPerKind >= CensusTopN || len(got.Reasons[0].TopPaths) != got.Bounds.TopPerKind {
		t.Errorf("bounds %+v, %d paths", got.Bounds, len(got.Reasons[0].TopPaths))
	}
	md, top, trunc := RenderCensusMarkdown(s)
	if len(md) > MaxSummaryMDBytes || top >= CensusTopN || trunc {
		t.Errorf("markdown: %d bytes, top %d, truncated %v", len(md), top, trunc)
	}
	// The last resort: even the headings do not fit.
	huge := &CensusSummary{Reasons: []KindSummary{{Kind: strings.Repeat("k", 300<<10)}}}
	md, _, trunc = RenderCensusMarkdown(huge)
	if !trunc || len(md) > MaxSummaryMDBytes || !strings.Contains(md, "cut to fit") {
		t.Errorf("hard truncation: %d bytes, %v", len(md), trunc)
	}
}

// The distinct-path cap is honoured and counted.
func TestCensusDistinctPathCap(t *testing.T) {
	a := newCensusAcc(nil)
	a.distinct = maxCensusDistinct - 1
	a.addCensus(&CensusDoc{Entries: []tracedEntry{
		{Kind: "host-read", Path: "/a", Count: 1}, {Kind: "host-read", Path: "/b", Count: 2}, {Kind: "host-read", Path: "/a", Count: 3},
	}})
	s := a.summary(10)
	if s.Bounds.PathsDropped != 1 || s.Reasons[0].DistinctPaths != 1 || s.Reasons[0].DroppedPaths != 2 || s.Reasons[0].TopPaths[0].Count != 4 {
		t.Errorf("%+v %+v", s.Bounds, s.Reasons[0])
	}
}

// More kinds than the cap collapse into "(other)".
func TestCensusKindCap(t *testing.T) {
	a := newCensusAcc(nil)
	var doc CensusDoc
	for i := 0; i < maxCensusKinds+5; i++ {
		doc.Entries = append(doc.Entries, tracedEntry{Kind: fmt.Sprintf("k%d", i), Path: "/p", Count: 1})
	}
	a.addCensus(&doc)
	s := a.summary(10)
	if len(s.Reasons) != maxCensusKinds+1 || s.Bounds.KindsCollapsed != 5 {
		t.Errorf("%d kinds, %d collapsed", len(s.Reasons), s.Bounds.KindsCollapsed)
	}
}

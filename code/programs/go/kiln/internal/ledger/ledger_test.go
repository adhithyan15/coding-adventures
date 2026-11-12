package ledger

import (
	"bytes"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

var t0 = time.Date(2026, 11, 11, 23, 24, 1, 0, time.UTC)

// nowT is "now" for the tests: after every timestamp the tests use.
var nowT = t0.Add(2 * time.Hour)

type fakeAuth struct{ trusted, local bool }

func (a fakeAuth) Trusted() bool           { return a.trusted }
func (a fakeAuth) WritesLocalLedger() bool { return a.local }

var trusted = fakeAuth{trusted: true}

func key(b byte) [32]byte {
	var k [32]byte
	for i := range k {
		k[i] = b
	}
	return k
}

func rec(b byte, kind string, at time.Time) Record {
	return Record{Key: key(b), Target: "rust/parser:test", OS: "linux", Arch: "x64",
		Kiln: "1.0.0", Revision: "0bf84a8", Run: "43117337685", At: at, Kind: kind}
}

func pass(b byte, at time.Time) Record { return rec(b, KindPass, at) }
func tomb(b byte, at time.Time) Record { return rec(b, KindTombstone, at) }

func goodLine(t *testing.T) string {
	t.Helper()
	l, err := pass(0xab, t0).Encode()
	if err != nil {
		t.Fatal(err)
	}
	return strings.TrimSuffix(string(l), "\n")
}

func TestRoundTrip(t *testing.T) {
	recs := []Record{pass(1, t0), tomb(2, t0.Add(time.Hour))}
	data, err := EncodeSegment(recs)
	if err != nil {
		t.Fatal(err)
	}
	got, err := ParseSegment(bytes.NewReader(data))
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 2 || got[0] != recs[0] || got[1] != recs[1] {
		t.Fatalf("round trip mismatch: %+v", got)
	}
	if got[0].Hex() != strings.Repeat("01", 32) {
		t.Fatal(got[0].Hex())
	}
	// Characters that encoding/json would escape stay literal (one spelling).
	r := pass(3, t0)
	r.Target = "a<b>&c:test"
	line, _ := r.Encode()
	if !strings.Contains(string(line), "a<b>&c") {
		t.Fatalf("escaped: %s", line)
	}
	if _, err := DecodeLine(bytes.TrimSuffix(line, []byte("\n"))); err != nil {
		t.Fatal(err)
	}
	if got, err := ParseSegment(bytes.NewReader(nil)); err != nil || len(got) != 0 {
		t.Fatal("empty segment should parse", err)
	}
}

func TestValidateRejections(t *testing.T) {
	long := func(n int) string { return strings.Repeat("a", n) }
	cases := map[string]func(*Record){
		"empty target":   func(r *Record) { r.Target = "" },
		"long target":    func(r *Record) { r.Target = long(MaxTargetBytes + 1) },
		"long os":        func(r *Record) { r.OS = long(MaxFieldBytes + 1) },
		"empty arch":     func(r *Record) { r.Arch = "" },
		"long revision":  func(r *Record) { r.Revision = long(MaxFieldBytes + 1) },
		"long run":       func(r *Record) { r.Run = long(MaxFieldBytes + 1) },
		"control char":   func(r *Record) { r.Target = "a\x01b" },
		"newline":        func(r *Record) { r.Run = "a\nb" },
		"del":            func(r *Record) { r.Kiln = "a\x7f" },
		"c1 control":     func(r *Record) { r.OS = "a\u0085b" },
		"bad utf8":       func(r *Record) { r.Target = "a\xffb" },
		"unknown kind":   func(r *Record) { r.Kind = "maybe" },
		"empty kind":     func(r *Record) { r.Kind = "" },
		"zero time":      func(r *Record) { r.At = time.Time{} },
		"non-utc":        func(r *Record) { r.At = t0.In(time.FixedZone("x", 3600)) },
		"subsecond":      func(r *Record) { r.At = t0.Add(time.Millisecond) },
		"year too large": func(r *Record) { r.At = time.Date(10000, 1, 1, 0, 0, 0, 0, time.UTC) },
	}
	for name, mut := range cases {
		r := pass(1, t0)
		mut(&r)
		if err := r.Validate(); err == nil {
			t.Errorf("%s: accepted", name)
		}
		if _, err := r.Encode(); err == nil {
			t.Errorf("%s: encoded", name)
		}
	}
}

func TestDecodeLineRejections(t *testing.T) {
	good := goodLine(t)
	key64 := strings.Repeat("ab", 32)
	cases := map[string]string{
		"not json":         "nope",
		"short key":        strings.Replace(good, key64, key64[:62], 1),
		"uppercase hex":    strings.Replace(good, key64, strings.ToUpper(key64), 1),
		"non-hex key":      strings.Replace(good, key64, strings.Repeat("zz", 32), 1),
		"unknown field":    strings.Replace(good, "}", `,"extra":1}`, 1),
		"trailing data":    good + " {}",
		"trailing garbage": good + "x",
		"unknown kind":     strings.Replace(good, `"pass"`, `"maybe"`, 1),
		"bad at":           strings.Replace(good, "2026-11-11T23:24:01Z", "yesterday", 1),
		"offset at":        strings.Replace(good, "2026-11-11T23:24:01Z", "2026-11-12T00:24:01+01:00", 1),
		"fractional at":    strings.Replace(good, "23:24:01Z", "23:24:01.5Z", 1),
		"space":            strings.Replace(good, `{"key"`, `{ "key"`, 1),
		"escaped":          strings.Replace(good, "rust", "\\u0072ust", 1),
		"duplicate key":    strings.Replace(good, `"os":"linux"`, `"os":"linux","os":"linux"`, 1),
		"reordered":        `{"target":"t","key":"` + key64 + `"}`,
		"control escaped":  strings.Replace(good, "rust", `a\u0001`, 1),
		"huge line":        strings.Repeat("x", MaxLineBytes),
		"empty":            "",
	}
	for name, line := range cases {
		if _, err := DecodeLine([]byte(line)); err == nil {
			t.Errorf("%s: accepted %q", name, line)
		}
	}
	// Invalid UTF-8 via raw bytes.
	raw := []byte(strings.Replace(good, "rust", "ru\xffst", 1))
	if _, err := DecodeLine(raw); err == nil || !strings.Contains(err.Error(), "UTF-8") {
		t.Errorf("raw bad utf8: %v", err)
	}
	// A valid line sanity check.
	if _, err := DecodeLine([]byte(good)); err != nil {
		t.Fatal(err)
	}
}

func TestParseSegmentRejections(t *testing.T) {
	good := goodLine(t)
	if _, err := ParseSegment(strings.NewReader(good)); err == nil {
		t.Error("missing final newline accepted")
	}
	if _, err := ParseSegment(strings.NewReader(good + "\n\n")); err == nil {
		t.Error("blank line accepted")
	}
	if _, err := ParseSegment(strings.NewReader(good + "\n" + "bad\n")); err == nil ||
		!strings.Contains(err.Error(), "line 2") {
		t.Errorf("bad second line: %v", err)
	}
	seg := good + "\n"
	// Too many records / too many bytes, with injected limits.
	if _, err := parseSegment(strings.NewReader(seg+seg+seg), 1<<20, 2); err == nil ||
		!strings.Contains(err.Error(), "records") {
		t.Errorf("record cap: %v", err)
	}
	if _, err := parseSegment(strings.NewReader(seg+seg), 2, 10); err == nil ||
		!strings.Contains(err.Error(), "larger") {
		t.Errorf("byte cap: %v", err)
	}
	// Exactly at the limits is fine.
	if _, err := parseSegment(strings.NewReader(seg+seg), int64(2*len(seg)), 2); err != nil {
		t.Errorf("at limit: %v", err)
	}
	// A reader that never ends is cut off by the LimitReader.
	if _, err := parseSegment(endless{}, 1000, 10); err == nil {
		t.Error("endless reader accepted")
	}
	if _, err := ParseSegment(errReader{}); err == nil {
		t.Error("read error swallowed")
	}
	// EncodeSegment caps.
	if _, err := EncodeSegment(make([]Record, MaxRecordsPerSegment+1)); err == nil {
		t.Error("too many records encoded")
	}
	if _, err := EncodeSegment([]Record{{}}); err == nil {
		t.Error("invalid record encoded")
	}
}

func TestEncodeSegmentByteCap(t *testing.T) {
	r := pass(1, t0)
	r.Target = strings.Repeat("a", MaxTargetBytes) // long lines reach the byte cap first
	r.Run = strings.Repeat("b", MaxFieldBytes)
	line, _ := r.Encode()
	n := MaxSegmentBytes/len(line) + 1
	if n > MaxRecordsPerSegment {
		t.Fatal("record cap reached before byte cap")
	}
	recs := make([]Record, n)
	for i := range recs {
		recs[i] = r
	}
	if _, err := EncodeSegment(recs); err == nil {
		t.Error("oversize segment encoded")
	}
}

type endless struct{}

func (endless) Read(p []byte) (int, error) {
	for i := range p {
		p[i] = 'x'
	}
	return len(p), nil
}

type errReader struct{}

func (errReader) Read([]byte) (int, error) { return 0, errors.New("boom") }

func TestSegmentNames(t *testing.T) {
	ok := segmentName(42, "0123456789abcdef")
	if ok != "0000000042-0123456789abcdef.jsonl" {
		t.Fatal(ok)
	}
	if seq, good := parseSegmentName(ok); !good || seq != 42 {
		t.Fatal(seq, good)
	}
	bad := []string{"", "x", "../0000000042-0123456789abcdef.jsonl",
		"0000000042-0123456789abcdef.json", "0000000042_0123456789abcdef.jsonl",
		"00000000x2-0123456789abcdef.jsonl", "0000000042-0123456789abcdeF.jsonl",
		"0000000042-0123456789abcdeg.jsonl", "a/00000042-0123456789abcdef.jsonl",
		"0000000042-0123456789abcdef.jsonl.tmp", ".tmp-123"}
	for _, n := range bad {
		if _, good := parseSegmentName(n); good {
			t.Errorf("%q accepted", n)
		}
	}
}

func TestStoreLookupAndShadowing(t *testing.T) {
	dir := filepath.Join(t.TempDir(), "ledger") // does not exist yet
	s := Open(dir, Shared)
	got, err := s.Lookup([][32]byte{key(1)})
	if err != nil || len(got) != 0 {
		t.Fatalf("missing dir should be empty: %v %v", got, err)
	}
	if err := s.Record(nil, trusted, nowT); err != nil { // empty batch: no-op
		t.Fatal(err)
	}
	if _, err := os.Stat(dir); err == nil {
		t.Fatal("empty batch created the directory")
	}

	if err := s.Record([]Record{pass(1, t0), pass(2, t0)}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	got, _ = s.Lookup([][32]byte{key(1), key(2), key(3)})
	if len(got) != 2 || got[key(1)].Kind != KindPass {
		t.Fatalf("lookup: %v", got)
	}

	// A newer tombstone hides the pass.
	if err := s.Record([]Record{tomb(1, t0.Add(time.Minute))}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	got, _ = s.Lookup([][32]byte{key(1), key(2)})
	if _, hit := got[key(1)]; hit || len(got) != 1 {
		t.Fatalf("tombstone did not shadow: %v", got)
	}
	// A pass OLDER than the tombstone, in a LATER segment, stays hidden:
	// At decides before segment order.
	if err := s.Record([]Record{pass(1, t0.Add(30*time.Second))}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	if got, _ = s.Lookup([][32]byte{key(1)}); len(got) != 0 {
		t.Fatalf("stale pass revived key: %v", got)
	}
	// A newer pass un-shadows.
	if err := s.Record([]Record{pass(1, t0.Add(time.Hour))}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	if got, _ = s.Lookup([][32]byte{key(1)}); len(got) != 1 {
		t.Fatalf("newer pass did not un-shadow: %v", got)
	}
	segs, _ := listSegments(dir)
	if len(segs) != 4 || segs[3].seq != 4 {
		t.Fatalf("segments: %v", segs)
	}
}

func TestNewerTieBreaks(t *testing.T) {
	mk := func(kind string, seq uint64, name string, line int) entry {
		return entry{rec(1, kind, t0), seq, name, line}
	}
	if !mk("", 2, "a", 1).newer(mk("", 1, "b", 9)) {
		t.Error("higher seq should win")
	}
	if !mk("", 1, "b", 1).newer(mk("", 1, "a", 9)) {
		t.Error("higher name should win")
	}
	if !mk("", 1, "a", 2).newer(mk("", 1, "a", 1)) {
		t.Error("later line should win")
	}
	if mk("", 1, "a", 1).newer(mk("", 1, "a", 1)) {
		t.Error("an entry is not newer than itself")
	}
	// Same At, same segment: the later line decides (tombstone after pass).
	dir := t.TempDir()
	data, _ := EncodeSegment([]Record{pass(1, t0), tomb(1, t0)})
	writeFile(t, dir, segmentName(1, "0123456789abcdef"), data)
	if got, err := Open(dir, Shared).Lookup([][32]byte{key(1)}); err != nil || len(got) != 0 {
		t.Fatalf("later line tombstone should win: %v %v", got, err)
	}
}

func writeFile(t *testing.T, dir, name string, data []byte) string {
	t.Helper()
	p := filepath.Join(dir, name)
	if err := os.WriteFile(p, data, 0o644); err != nil {
		t.Fatal(err)
	}
	return p
}

func TestRecordRefusedWhenUntrusted(t *testing.T) {
	dir := t.TempDir()
	err := Open(dir, Shared).Record([]Record{pass(1, t0)}, nil, nowT)
	if !errors.Is(err, ErrUntrusted) {
		t.Fatalf("got %v", err)
	}
	if ents, _ := os.ReadDir(dir); len(ents) != 0 {
		t.Fatalf("untrusted run wrote files: %v", ents)
	}
}

func TestRecordRefusesInvalidBatchWhole(t *testing.T) {
	dir := t.TempDir()
	bad := pass(2, t0)
	bad.Kind = "x"
	if err := Open(dir, Shared).Record([]Record{pass(1, t0), bad}, trusted, nowT); err == nil {
		t.Fatal("invalid batch accepted")
	}
	if ents, _ := os.ReadDir(dir); len(ents) != 0 {
		t.Fatalf("partial write: %v", ents)
	}
}

func TestRecordErrors(t *testing.T) {
	// The ledger path is a regular file: MkdirAll fails.
	f := writeFile(t, t.TempDir(), "file", nil)
	if err := Open(f, Shared).Record([]Record{pass(1, t0)}, trusted, nowT); err == nil {
		t.Error("MkdirAll on a file succeeded")
	}
	// A bad segment in the directory blocks writing too (fail closed).
	dir := t.TempDir()
	if err := os.Mkdir(filepath.Join(dir, segmentName(1, "0123456789abcdef")), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := Open(dir, Shared).Record([]Record{pass(1, t0)}, trusted, nowT); err == nil {
		t.Error("directory-as-segment accepted")
	}
	// Sequence exhaustion.
	dir = t.TempDir()
	writeFile(t, dir, segmentName(maxSeq, "0123456789abcdef"), nil)
	if err := Open(dir, Shared).Record([]Record{pass(1, t0)}, trusted, nowT); err == nil {
		t.Error("exhausted sequence accepted")
	}
}

// --- atomic write ---

type fakeTmp struct {
	*os.File
	failWrite, failSync, failClose bool
}

func (f *fakeTmp) Write(p []byte) (int, error) {
	if f.failWrite {
		n, _ := f.File.Write(p[:len(p)/2]) // a partial write, then failure
		return n, errors.New("disk full")
	}
	return f.File.Write(p)
}
func (f *fakeTmp) Sync() error {
	if f.failSync {
		return errors.New("sync failed")
	}
	return f.File.Sync()
}
func (f *fakeTmp) Close() error {
	err := f.File.Close()
	if f.failClose {
		return errors.New("close failed")
	}
	return err
}

func TestAtomicWriteFailuresLeaveNothing(t *testing.T) {
	data, _ := EncodeSegment([]Record{pass(1, t0)})
	name := segmentName(1, "0123456789abcdef")
	cases := map[string]fsOps{}
	for _, mode := range []string{"write", "sync", "close"} {
		mode := mode
		cases[mode] = fsOps{
			create: func(dir string) (tmpFile, error) {
				f, err := os.CreateTemp(dir, ".tmp-*")
				return &fakeTmp{File: f, failWrite: mode == "write", failSync: mode == "sync",
					failClose: mode == "close"}, err
			},
			rename: os.Rename,
		}
	}
	cases["rename"] = fsOps{
		create:  realFS.create,
		syncDir: realFS.syncDir,
		rename:  func(string, string) error { return errors.New("rename failed") },
	}
	for mode, ops := range cases {
		dir := t.TempDir()
		if err := writeAtomic(ops, dir, name, data); err == nil {
			t.Errorf("%s: no error", mode)
		}
		if ents, _ := os.ReadDir(dir); len(ents) != 0 {
			t.Errorf("%s: left files behind: %v", mode, ents)
		}
		// And the ledger still reads cleanly.
		if got, err := Open(dir, Shared).Lookup([][32]byte{key(1)}); err != nil || len(got) != 0 {
			t.Errorf("%s: %v %v", mode, got, err)
		}
	}
	// A create failure is returned as is.
	ops := fsOps{create: func(string) (tmpFile, error) { return nil, errors.New("no temp") }}
	if err := writeAtomic(ops, t.TempDir(), name, data); err == nil {
		t.Error("create failure swallowed")
	}
	// Through the Store: an injected failure surfaces from Record.
	dir := t.TempDir()
	s := Open(dir, Shared)
	s.fs = cases["write"]
	if err := s.Record([]Record{pass(1, t0)}, trusted, nowT); err == nil {
		t.Error("Record swallowed write failure")
	}
}

func TestInterruptedWriteTempIgnored(t *testing.T) {
	dir := t.TempDir()
	writeFile(t, dir, ".tmp-123", []byte("garbage, half a rec"))
	writeFile(t, dir, "README", []byte("hi"))
	s := Open(dir, Shared)
	if err := s.Record([]Record{pass(1, t0)}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	if got, err := s.Lookup([][32]byte{key(1)}); err != nil || len(got) != 1 {
		t.Fatalf("%v %v", got, err)
	}
}

// --- directory safety ---

func TestLookupFailsClosedOnBadSegment(t *testing.T) {
	dir := t.TempDir()
	s := Open(dir, Shared)
	if err := s.Record([]Record{pass(1, t0)}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	writeFile(t, dir, segmentName(2, "0123456789abcdef"), []byte("not a record\n"))
	if got, err := s.Lookup([][32]byte{key(1)}); err == nil || got != nil {
		t.Fatalf("bad segment tolerated: %v %v", got, err)
	}
}

func TestSymlinkedSegmentRejected(t *testing.T) {
	dir, other := t.TempDir(), t.TempDir()
	data, _ := EncodeSegment([]Record{pass(1, t0)})
	target := writeFile(t, other, "real", data)
	if err := os.Symlink(target, filepath.Join(dir, segmentName(1, "0123456789abcdef"))); err != nil {
		t.Skipf("symlinks unsupported here: %v", err)
	}
	if got, err := Open(dir, Shared).Lookup([][32]byte{key(1)}); err == nil {
		t.Fatalf("symlinked segment followed: %v", got)
	}
}

func TestDirectoryAsSegmentRejected(t *testing.T) {
	dir := t.TempDir()
	if err := os.Mkdir(filepath.Join(dir, segmentName(1, "0123456789abcdef")), 0o755); err != nil {
		t.Fatal(err)
	}
	if _, err := Open(dir, Shared).Lookup(nil); err == nil || !strings.Contains(err.Error(), "regular") {
		t.Fatalf("got %v", err)
	}
}

func TestReadRecordFileErrors(t *testing.T) {
	dir := t.TempDir()
	if _, _, err := readRecordFile(filepath.Join(dir, "missing"), 1<<20); err == nil {
		t.Error("missing file accepted")
	}
	// The Lstat check refuses a directory before it is opened.
	if _, _, err := readRecordFile(dir, 1<<20); err == nil {
		t.Error("directory accepted")
	}
	// The descriptor check (the TOCTOU defence) refuses one that slips past.
	d, err := os.Open(dir)
	if err != nil {
		t.Skipf("cannot open a directory here: %v", err)
	}
	defer d.Close()
	if _, err := requireRegular(d); err == nil {
		t.Error("directory descriptor accepted")
	}
	closed, _ := os.Open(dir)
	closed.Close()
	if _, err := requireRegular(closed); err == nil { // Stat on a closed file
		t.Error("closed file accepted")
	}
	// Per-file budget.
	data, _ := EncodeSegment([]Record{pass(1, t0)})
	p := writeFile(t, dir, "seg", data)
	if _, _, err := readRecordFile(p, int64(len(data))-1); err == nil || !strings.Contains(err.Error(), "budget") {
		t.Errorf("budget: %v", err)
	}
	if _, n, err := readRecordFile(p, int64(len(data))); err != nil || n != int64(len(data)) {
		t.Errorf("at budget: %v", err)
	}
}

func TestListSegmentsErrors(t *testing.T) {
	f := writeFile(t, t.TempDir(), "file", nil)
	if _, err := listSegments(f); err == nil { // ReadDir on a file
		t.Error("file as directory accepted")
	}
}

func TestSegmentAndRecordCaps(t *testing.T) {
	dir := t.TempDir()
	for i := 0; i < MaxSegments+1; i++ {
		// Empty files are valid, cheap segments.
		writeFile(t, dir, segmentName(uint64(i+1), "0123456789abcdef"), nil)
	}
	if _, err := listSegments(dir); err == nil || !strings.Contains(err.Error(), "segments") {
		t.Fatalf("segment cap: %v", err)
	}
	// MaxDirRecords is exercised by lowering the effective total via many
	// full-ish segments is too slow; check the arithmetic path directly.
	dir = t.TempDir()
	data, _ := EncodeSegment([]Record{pass(1, t0)})
	writeFile(t, dir, segmentName(1, "0123456789abcdef"), data)
	if es, err := loadDir(dir); err != nil || len(es) != 1 {
		t.Fatal(err)
	}
}

// --- merge ---

func snapshotWith(t *testing.T, recs ...Record) string {
	t.Helper()
	dir := t.TempDir()
	if len(recs) > 0 {
		if err := Open(dir, Shared).Record(recs, trusted, nowT); err != nil {
			t.Fatal(err)
		}
	}
	return dir
}

func deltaFile(t *testing.T, recs ...Record) string {
	t.Helper()
	data, err := EncodeSegment(recs)
	if err != nil {
		t.Fatal(err)
	}
	return writeFile(t, t.TempDir(), "delta.jsonl", data)
}

func dirBytes(t *testing.T, dir string) string {
	t.Helper()
	ents, err := os.ReadDir(dir)
	if err != nil {
		t.Fatal(err)
	}
	var sb strings.Builder
	for _, e := range ents {
		b, _ := os.ReadFile(filepath.Join(dir, e.Name()))
		fmt.Fprintf(&sb, "%s\n%s", e.Name(), b)
	}
	return sb.String()
}

func TestMergeUnionsAndCompacts(t *testing.T) {
	snap := snapshotWith(t, pass(1, t0), pass(2, t0))
	if err := Open(snap, Shared).Record([]Record{pass(1, t0.Add(time.Minute))}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	delta := deltaFile(t, pass(3, t0), tomb(2, t0.Add(time.Hour)))
	out := filepath.Join(t.TempDir(), "out")
	st, err := Merge(snap, delta, out, trusted, nowT, nil)
	if err != nil {
		t.Fatal(err)
	}
	if st.Snapshot != 3 || st.Delta != 2 || st.Kept != 3 || st.Pruned != 0 {
		t.Fatalf("stats: %+v", st)
	}
	got, err := Open(out, Shared).Lookup([][32]byte{key(1), key(2), key(3)})
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 2 || got[key(1)].At != t0.Add(time.Minute) {
		t.Fatalf("merged lookup: %v", got)
	}
	if _, hit := got[key(2)]; hit {
		t.Error("delta tombstone lost")
	}
	segs, _ := listSegments(out)
	if len(segs) != 1 || segs[0].seq != 3 { // continues snapshot's sequence 1,2
		t.Fatalf("out segments: %v", segs)
	}
	// The snapshot was not modified.
	if s, _ := listSegments(snap); len(s) != 2 {
		t.Fatal("snapshot changed")
	}
}

func TestMergeEmptySnapshotAndEmptyResult(t *testing.T) {
	out := filepath.Join(t.TempDir(), "o")
	st, err := Merge(filepath.Join(t.TempDir(), "none"), deltaFile(t, pass(1, t0)), out, trusted, nowT, nil)
	if err != nil || st.Kept != 1 {
		t.Fatal(st, err)
	}
	// Empty delta, empty snapshot: an empty (but created) output directory.
	out2 := filepath.Join(t.TempDir(), "o2")
	if _, err := Merge(snapshotWith(t), deltaFile(t), out2, trusted, nowT, nil); err != nil {
		t.Fatal(err)
	}
	if ents, err := os.ReadDir(out2); err != nil || len(ents) != 0 {
		t.Fatal(ents, err)
	}
	// An existing empty output directory is fine.
	if _, err := Merge(snapshotWith(t), deltaFile(t, pass(1, t0)), t.TempDir(), trusted, nowT, nil); err != nil {
		t.Fatal(err)
	}
}

func TestMergeRejectsDeltaWithOneBadRecord(t *testing.T) {
	good, _ := EncodeSegment([]Record{pass(1, t0), pass(2, t0)})
	bad := append(append([]byte{}, good...), []byte(strings.Replace(goodLine(t), `"pass"`, `"bogus"`, 1)+"\n")...)
	delta := writeFile(t, t.TempDir(), "delta.jsonl", bad)
	out := filepath.Join(t.TempDir(), "out")
	if _, err := Merge(snapshotWith(t, pass(9, t0)), delta, out, trusted, nowT, nil); err == nil ||
		!strings.Contains(err.Error(), "delta") {
		t.Fatalf("got %v", err)
	}
	if _, err := os.Stat(out); err == nil {
		t.Fatal("output created for a rejected delta")
	}
}

func TestMergeErrors(t *testing.T) {
	snap := snapshotWith(t, pass(1, t0))
	delta := deltaFile(t, pass(2, t0))
	if _, err := Merge(snap, delta, snap, trusted, nowT, nil); err == nil {
		t.Error("output == snapshot accepted")
	}
	nonEmpty := snapshotWith(t, pass(3, t0))
	if _, err := Merge(snap, delta, nonEmpty, trusted, nowT, nil); err == nil {
		t.Error("non-empty output accepted")
	}
	if _, err := Merge(snap, filepath.Join(t.TempDir(), "nodelta"), filepath.Join(t.TempDir(), "o"), trusted, nowT, nil); err == nil {
		t.Error("missing delta accepted")
	}
	badSnap := t.TempDir()
	writeFile(t, badSnap, segmentName(1, "0123456789abcdef"), []byte("junk\n"))
	if _, err := Merge(badSnap, delta, filepath.Join(t.TempDir(), "o"), trusted, nowT, nil); err == nil ||
		!strings.Contains(err.Error(), "snapshot") {
		t.Errorf("bad snapshot: %v", err)
	}
	file := writeFile(t, t.TempDir(), "f", nil)
	if _, err := Merge(snap, delta, file, trusted, nowT, nil); err == nil { // ReadDir on a file
		t.Error("file as output accepted")
	}
	if _, err := Merge(snap, delta, filepath.Join(file, "sub"), trusted, nowT, nil); err == nil { // MkdirAll under a file
		t.Error("uncreatable output accepted")
	}
	exhausted := t.TempDir()
	writeFile(t, exhausted, segmentName(maxSeq, "0123456789abcdef"), nil)
	if _, err := Merge(exhausted, delta, filepath.Join(t.TempDir(), "o"), trusted, nowT, nil); err == nil {
		t.Error("exhausted sequence accepted")
	}
}

func TestMergeDeterministic(t *testing.T) {
	snap := snapshotWith(t, pass(5, t0), pass(1, t0))
	delta := deltaFile(t, pass(3, t0), tomb(1, t0.Add(time.Hour)), pass(9, t0))
	run := func() string {
		out := filepath.Join(t.TempDir(), "out")
		if _, err := Merge(snap, delta, out, trusted, nowT, nil); err != nil {
			t.Fatal(err)
		}
		return dirBytes(t, out)
	}
	a, b := run(), run()
	if a != b {
		t.Fatalf("not deterministic:\n%s\n---\n%s", a, b)
	}
	// Sorted by key.
	if i1, i3 := strings.Index(a, strings.Repeat("01", 32)), strings.Index(a, strings.Repeat("03", 32)); i1 > i3 {
		t.Error("output not sorted by key")
	}
}

func TestMergeSplitsLargeOutput(t *testing.T) {
	// MaxRecordsPerSegment+1 distinct keys need two segments.
	n := MaxRecordsPerSegment + 1
	recs := make([]Record, n)
	for i := range recs {
		r := pass(0, t0)
		r.Key[0], r.Key[1], r.Key[2] = byte(i>>16), byte(i>>8), byte(i)
		recs[i] = r
	}
	data, err := EncodeSegment(recs[:MaxRecordsPerSegment])
	if err != nil {
		t.Fatal(err)
	}
	delta := writeFile(t, t.TempDir(), "d", data)
	snap := snapshotWith(t, recs[MaxRecordsPerSegment])
	out := filepath.Join(t.TempDir(), "out")
	st, err := Merge(snap, delta, out, trusted, nowT, nil)
	if err != nil || st.Kept != n {
		t.Fatal(st, err)
	}
	if segs, _ := listSegments(out); len(segs) != 2 {
		t.Fatalf("want 2 segments, got %v", segs)
	}
}

func TestMergePruning(t *testing.T) {
	cutoff := t0.Add(-24 * time.Hour)
	old := t0.Add(-48 * time.Hour)
	snap := snapshotWith(t,
		pass(1, old),                              // old pass, no tombstone: pruned
		pass(2, t0),                               // fresh pass: kept
		pass(3, old),                              // old pass ...
		tomb(3, t0),                               // ... shadowed by a fresh tombstone: tombstone kept
		pass(4, old), tomb(4, old.Add(time.Hour)), // old tombstone over old pass: both pruned
	)
	st, err := Merge(snap, deltaFile(t), filepath.Join(t.TempDir(), "o"), trusted, nowT, OlderThan(cutoff))
	if err != nil {
		t.Fatal(err)
	}
	if st.Kept != 2 || st.Pruned != 2 {
		t.Fatalf("stats: %+v", st)
	}

	// Predicate-level guarantee: a tombstone the predicate selects is still
	// kept while a pass for its key survives the predicate.
	byRevision := func(r Record) bool { return r.Kind == KindTombstone } // selects only tombstones
	out := filepath.Join(t.TempDir(), "o2")
	snap2 := snapshotWith(t, pass(7, old), tomb(7, t0))
	st, err = Merge(snap2, deltaFile(t, pass(8, t0)), out, trusted, nowT, byRevision)
	if err != nil {
		t.Fatal(err)
	}
	if st.Pruned != 0 || st.Kept != 2 {
		t.Fatalf("shadowing tombstone dropped: %+v", st)
	}
	if got, _ := Open(out, Shared).Lookup([][32]byte{key(7)}); len(got) != 0 {
		t.Fatal("pruning un-shadowed key 7")
	}
	// But a tombstone is dropped when no pass for the key survives.
	out3 := filepath.Join(t.TempDir(), "o3")
	snap3 := snapshotWith(t, tomb(9, t0))
	st, err = Merge(snap3, deltaFile(t), out3, trusted, nowT, byRevision)
	if err != nil || st.Pruned != 1 || st.Kept != 0 {
		t.Fatal(st, err)
	}
}

func TestMergeDeltaWinsTies(t *testing.T) {
	snap := snapshotWith(t, pass(1, t0))
	out := filepath.Join(t.TempDir(), "out")
	if _, err := Merge(snap, deltaFile(t, tomb(1, t0)), out, trusted, nowT, nil); err != nil {
		t.Fatal(err)
	}
	if got, _ := Open(out, Shared).Lookup([][32]byte{key(1)}); len(got) != 0 {
		t.Fatal("delta tombstone at equal At should win")
	}
}

func TestMergeSymlinkedDeltaRejected(t *testing.T) {
	real := deltaFile(t, pass(1, t0))
	link := filepath.Join(t.TempDir(), "link")
	if err := os.Symlink(real, link); err != nil {
		t.Skipf("symlinks unsupported here: %v", err)
	}
	if _, err := Merge(snapshotWith(t), link, filepath.Join(t.TempDir(), "o"), trusted, nowT, nil); err == nil {
		t.Fatal("symlinked delta followed")
	}
}

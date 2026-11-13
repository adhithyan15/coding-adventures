package ledger

import (
	"bytes"
	"crypto/sha256"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"
)

// mf builds a valid manifest.  w picks the weak key; the entries default to a
// small valid set; at is the record time and run the CI run id.
func mf(w byte, run string, at time.Time, entries ...string) Manifest {
	if len(entries) == 0 {
		entries = []string{"list:code/packages/go", "path:go.work", "pkg:code/packages/go/lexer"}
	}
	return Manifest{Weak: key(w), Target: "code/packages/go/parser", OS: "linux", Arch: "x64",
		Entries: entries, Revision: "0bf84a8", Run: run, At: at}
}

func mfLine(t *testing.T, m Manifest) string {
	t.Helper()
	l, err := m.Encode()
	if err != nil {
		t.Fatal(err)
	}
	return strings.TrimSuffix(string(l), "\n")
}

func manifestDirOf(t *testing.T) string { return filepath.Join(t.TempDir(), ManifestsDir) }

// writeManifestSeg writes lines as a segment file named by seq.
func writeManifestSeg(t *testing.T, dir string, seq uint64, lines ...string) string {
	t.Helper()
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	name := segmentName(seq, "00000000000000aa")
	var b strings.Builder
	for _, l := range lines {
		b.WriteString(l + "\n")
	}
	return writeFile(t, dir, name, []byte(b.String()))
}

func load(t *testing.T, dir string, now time.Time) (*ManifestSet, ManifestReport, error) {
	t.Helper()
	// dir is the manifests directory itself; OpenManifests wants its parent.
	return OpenManifests(filepath.Dir(dir), Shared).Load(now)
}

func TestManifestRoundTripAndGolden(t *testing.T) {
	m := mf(0xab, "43117337685", t0)
	line, err := m.Encode()
	if err != nil {
		t.Fatal(err)
	}
	id := m.ID()
	want := fmt.Sprintf(`{"kind":"manifest","id":"%x","weak":"%x","target":"code/packages/go/parser",`+
		`"os":"linux","arch":"x64","entries":["list:code/packages/go","path:go.work","pkg:code/packages/go/lexer"],`+
		`"revision":"0bf84a8","run":"43117337685","at":"2026-11-11T23:24:01Z"}`+"\n", id[:], key(0xab))
	if string(line) != want {
		t.Fatalf("encoding\n got %s\nwant %s", line, want)
	}
	// The id is the SHA-256 of the compact entries array.
	sum := sha256.Sum256([]byte(`["list:code/packages/go","path:go.work","pkg:code/packages/go/lexer"]`))
	if id != sum {
		t.Error("id is not the SHA-256 of the canonical entries list")
	}
	got, err := decodeManifestLine(line[:len(line)-1])
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(got, m) {
		t.Errorf("round trip: got %+v want %+v", got, m)
	}
	if m.WeakHex() != strings.Repeat("ab", 32) {
		t.Error("WeakHex")
	}
}

func TestManifestEmptyEntriesAndRoot(t *testing.T) {
	m := mf(1, "1", t0)
	m.Entries = nil
	line := mfLine(t, m)
	if !strings.Contains(line, `"entries":[]`) {
		t.Errorf("nil entries must encode as []: %s", line)
	}
	if _, err := decodeManifestLine([]byte(line)); err != nil {
		t.Error(err)
	}
	if m.ID() != (Manifest{Entries: []string{}}).ID() {
		t.Error("nil and empty entries must share an id")
	}
	root := mf(1, "1", t0, "list:.", "path:a")
	if err := root.Validate(); err != nil {
		t.Errorf("list:. refused: %v", err)
	}
	// "." is only special for list:.
	for _, e := range []string{"path:.", "pkg:."} {
		if err := mf(1, "1", t0, e).Validate(); err == nil {
			t.Errorf("%s accepted", e)
		}
	}
}

func TestManifestValidateRejections(t *testing.T) {
	bad := map[string]func(*Manifest){
		"empty target":         func(m *Manifest) { m.Target = "" },
		"absolute target":      func(m *Manifest) { m.Target = "/a/b" },
		"dotdot target":        func(m *Manifest) { m.Target = "a/../b" },
		"trailing slash":       func(m *Manifest) { m.Target = "a/b/" },
		"long target":          func(m *Manifest) { m.Target = strings.Repeat("a", MaxTargetBytes+1) },
		"control in target":    func(m *Manifest) { m.Target = "a\x01b" },
		"empty os":             func(m *Manifest) { m.OS = "" },
		"empty arch":           func(m *Manifest) { m.Arch = "" },
		"long revision":        func(m *Manifest) { m.Revision = strings.Repeat("r", MaxFieldBytes+1) },
		"format char in run":   func(m *Manifest) { m.Run = "a\u202eb" },
		"zero at":              func(m *Manifest) { m.At = time.Time{} },
		"non-UTC at":           func(m *Manifest) { m.At = t0.In(time.FixedZone("x", 3600)) },
		"fractional at":        func(m *Manifest) { m.At = t0.Add(time.Nanosecond) },
		"year out of range":    func(m *Manifest) { m.At = time.Date(10000, 1, 1, 0, 0, 0, 0, time.UTC) },
		"no prefix":            func(m *Manifest) { m.Entries = []string{"go.work"} },
		"disallowed prefix":    func(m *Manifest) { m.Entries = []string{"env:HOME"} },
		"host prefix":          func(m *Manifest) { m.Entries = []string{"host-read:/etc/passwd"} },
		"empty entry":          func(m *Manifest) { m.Entries = []string{""} },
		"absolute path entry":  func(m *Manifest) { m.Entries = []string{"path:/etc/passwd"} },
		"dotdot path entry":    func(m *Manifest) { m.Entries = []string{"path:a/../b"} },
		"empty path entry":     func(m *Manifest) { m.Entries = []string{"path:"} },
		"empty list entry":     func(m *Manifest) { m.Entries = []string{"list:"} },
		"slash-only list":      func(m *Manifest) { m.Entries = []string{"list:a//b"} },
		"newline in entry":     func(m *Manifest) { m.Entries = []string{"path:a\nb"} },
		"unsorted":             func(m *Manifest) { m.Entries = []string{"path:b", "path:a"} },
		"duplicate":            func(m *Manifest) { m.Entries = []string{"path:a", "path:a"} },
		"invalid UTF-8 entry":  func(m *Manifest) { m.Entries = []string{"path:\xff"} },
		"prefix case matters":  func(m *Manifest) { m.Entries = []string{"PKG:a"} },
		"prefix only":          func(m *Manifest) { m.Entries = []string{"pkg"} },
		"windows-ish prefixes": func(m *Manifest) { m.Entries = []string{"c:/a"} },
	}
	for name, mutate := range bad {
		m := mf(1, "1", t0)
		mutate(&m)
		if err := m.Validate(); err == nil {
			t.Errorf("%s: accepted", name)
		}
		if _, err := m.Encode(); err == nil {
			t.Errorf("%s: encoded", name)
		}
	}
	// Overlong entry names are a CAP, not a structure error.
	long := mf(1, "1", t0, "path:"+strings.Repeat("a", MaxManifestNameBytes))
	if err := long.validateStructure(); err != nil {
		t.Errorf("long name is structurally fine: %v", err)
	}
	if err := long.Validate(); !errors.Is(err, ErrManifestTooLarge) {
		t.Errorf("long name: %v", err)
	}
	// Exactly at the caps is fine.
	atCap := mf(1, "1", t0, "path:"+strings.Repeat("a", MaxManifestNameBytes-5))
	if _, err := atCap.Encode(); err != nil {
		t.Errorf("name at cap: %v", err)
	}
}

func manyEntries(n, width int) []string {
	out := make([]string, n)
	for i := range out {
		out[i] = fmt.Sprintf("path:%0*d", width, i)
	}
	return out
}

func TestManifestCapsAtEmit(t *testing.T) {
	// Entry count.
	over := mf(1, "1", t0)
	over.Entries = manyEntries(MaxManifestEntries+1, 6)
	if _, err := over.Encode(); !errors.Is(err, ErrManifestTooLarge) {
		t.Errorf("entry count: %v", err)
	}
	atCap := mf(1, "1", t0)
	atCap.Entries = manyEntries(MaxManifestEntries, 6)
	if _, err := atCap.Encode(); err != nil {
		t.Errorf("8192 entries: %v", err)
	}
	// Line length: 8192 entries of 200 bytes is over 1 MiB with each name legal.
	wide := mf(1, "1", t0)
	wide.Entries = manyEntries(MaxManifestEntries, 195)
	if _, err := wide.Encode(); !errors.Is(err, ErrManifestTooLarge) || !strings.Contains(err.Error(), "line is") {
		t.Errorf("line length: %v", err)
	}
	// Record refuses the whole batch and writes nothing.
	dir := t.TempDir()
	s := OpenManifests(dir, Shared)
	err := s.Record([]Manifest{mf(1, "1", t0), wide}, trusted, nowT)
	if !errors.Is(err, ErrManifestTooLarge) {
		t.Errorf("Record: %v", err)
	}
	if _, statErr := os.Stat(filepath.Join(dir, ManifestsDir)); statErr == nil {
		t.Error("refused batch created the directory")
	}
}

func TestDecodeManifestLineRefusals(t *testing.T) {
	m := mf(0xcd, "9", t0)
	good := mfLine(t, m)
	id := m.ID()
	idHex := fmt.Sprintf("%x", id[:])
	badID := strings.Repeat("0", 64)
	cases := map[string]string{
		"empty":             "",
		"not json":          "nope",
		"unknown field":     strings.Replace(good, `"run":`, `"extra":1,"run":`, 1),
		"trailing data":     good + " {}",
		"two values":        good + good,
		"whitespace":        strings.Replace(good, `,"os"`, `, "os"`, 1),
		"reordered":         `{"id":"` + idHex + `",` + strings.TrimPrefix(good, `{"kind":"manifest","id":"`+idHex+`",`),
		"id mismatch":       strings.Replace(good, idHex, badID, 1),
		"uppercase id":      strings.Replace(good, idHex, strings.ToUpper(idHex), 1),
		"short id":          strings.Replace(good, idHex, idHex[:60], 1),
		"short weak":        strings.Replace(good, m.WeakHex(), m.WeakHex()[:62], 1),
		"non-hex weak":      strings.Replace(good, m.WeakHex(), strings.Repeat("g", 64), 1),
		"wrong kind":        strings.Replace(good, `"manifest"`, `"pass"`, 1),
		"missing kind":      strings.Replace(good, `"kind":"manifest",`, ``, 1),
		"null entries":      strings.Replace(good, `["list:code/packages/go","path:go.work","pkg:code/packages/go/lexer"]`, `null`, 1),
		"bad prefix":        strings.Replace(good, `"path:go.work"`, `"env:HOME"`, 1),
		"unsorted entries":  strings.Replace(good, `["list:code/packages/go","path:go.work"`, `["path:go.work","list:code/packages/go"`, 1),
		"escaped entry":     strings.Replace(good, `"path:go.work"`, `"path:go\u002ework"`, 1),
		"fractional second": strings.Replace(good, `23:24:01Z`, `23:24:01.5Z`, 1),
		"offset time":       strings.Replace(good, `23:24:01Z`, `23:24:01+00:00`, 1),
		"bad time":          strings.Replace(good, `2026-11-11T23:24:01Z`, `yesterday`, 1),
		"numeric field":     strings.Replace(good, `"run":"9"`, `"run":9`, 1),
		"invalid UTF-8":     strings.Replace(good, `"linux"`, "\"li\xffnux\"", 1),
	}
	for name, line := range cases {
		if _, err := decodeManifestLine([]byte(line)); err == nil {
			t.Errorf("%s: accepted", name)
		} else if errors.Is(err, errOverCap) {
			t.Errorf("%s: classed as over-cap, want structural: %v", name, err)
		}
	}
	// The id mismatch gets its own message.
	_, err := decodeManifestLine([]byte(cases["id mismatch"]))
	if err == nil || !strings.Contains(err.Error(), "id does not match") {
		t.Errorf("id mismatch message: %v", err)
	}
	// A hand-built non-canonical but otherwise valid spelling.
	if _, err := decodeManifestLine([]byte(cases["whitespace"])); err == nil || !strings.Contains(err.Error(), "canonical") {
		t.Errorf("whitespace message: %v", err)
	}
	if _, err := decodeManifestLine([]byte(good)); err != nil {
		t.Errorf("good line refused: %v", err)
	}
}

func TestPassSegmentLineIsNotAManifest(t *testing.T) {
	pl, _ := pass(1, t0).Encode()
	if _, err := decodeManifestLine(bytes.TrimSuffix(pl, []byte("\n"))); err == nil {
		t.Error("a pass line decoded as a manifest")
	}
}

// overCapLine returns the canonical encoding of a record that is over a
// per-record cap, built by bypassing Encode's refusal, as a hostile or buggy
// writer would.
func overCapLine(m Manifest) string { return strings.TrimSuffix(string(m.encode()), "\n") }

func TestReaderDropsOverCapRecordsAndKeepsTheRest(t *testing.T) {
	dir := manifestDirOf(t)
	tooMany := mf(2, "2", t0)
	tooMany.Entries = manyEntries(MaxManifestEntries+1, 6)
	longName := mf(3, "3", t0, "path:"+strings.Repeat("a", MaxManifestNameBytes))
	wide := mf(4, "4", t0)
	wide.Entries = manyEntries(MaxManifestEntries, 195)
	garbageHuge := strings.Repeat("x", MaxManifestLineBytes) // never parsed: dropped on length
	writeManifestSeg(t, dir, 1, mfLine(t, mf(1, "1", t0)), overCapLine(tooMany), overCapLine(longName),
		overCapLine(wide), garbageHuge, mfLine(t, mf(5, "5", t0)))

	set, rep, err := load(t, dir, nowT)
	if err != nil {
		t.Fatalf("over-cap records must not make the store unusable: %v", err)
	}
	if len(rep.Dropped) != 4 {
		t.Fatalf("dropped = %+v", rep.Dropped)
	}
	for i, wantLine := range []int{2, 3, 4, 5} {
		if rep.Dropped[i].Line != wantLine || rep.Dropped[i].Segment == "" || rep.Dropped[i].Reason == "" {
			t.Errorf("drop %d: %+v", i, rep.Dropped[i])
		}
	}
	if rep.Read != 2 || rep.Retained != 2 {
		t.Errorf("report %+v", rep)
	}
	if len(set.Manifests(key(1))) != 1 || len(set.Manifests(key(5))) != 1 {
		t.Error("good records lost")
	}
	for _, w := range []byte{2, 3, 4} {
		if set.Manifests(key(w)) != nil {
			t.Errorf("over-cap record %d served", w)
		}
	}
	// The same lines classify as over-cap, not structural, one by one.
	for _, m := range []Manifest{tooMany, longName} {
		if _, err := decodeManifestLine([]byte(overCapLine(m))); !errors.Is(err, errOverCap) {
			t.Errorf("want over-cap: %v", err)
		}
	}
}

func TestOverCapAndNonCanonicalIsStructural(t *testing.T) {
	// A record that is over a cap AND not in canonical form is corruption,
	// never a quiet drop: structure is judged first.
	tooMany := mf(2, "2", t0)
	tooMany.Entries = manyEntries(MaxManifestEntries+1, 6)
	line := strings.Replace(overCapLine(tooMany), `"os":`, `"os" :`, 1)
	dir := manifestDirOf(t)
	writeManifestSeg(t, dir, 1, mfLine(t, mf(1, "1", t0)), line)
	if set, _, err := load(t, dir, nowT); err == nil || set != nil {
		t.Errorf("want unusable, got set=%v err=%v", set, err)
	}
}

func TestReaderStructuralCorruptionMakesStoreUnusable(t *testing.T) {
	good := mfLine(t, mf(1, "1", t0))
	m2 := mf(2, "2", t0)
	cases := map[string][]string{
		"non-canonical": {good, strings.Replace(mfLine(t, m2), `,"os"`, `, "os"`, 1)},
		"unknown field": {good, strings.Replace(mfLine(t, m2), `"run":`, `"x":0,"run":`, 1)},
		"id mismatch":   {good, strings.Replace(mfLine(t, m2), fmt.Sprintf("%x", func() []byte { i := m2.ID(); return i[:] }()), strings.Repeat("1", 64), 1)},
		"garbage":       {good, "not json"},
		"blank line":    {good, "", good},
		"bad entry":     {good, strings.Replace(mfLine(t, m2), `"path:go.work"`, `"env:X"`, 1)},
	}
	for name, lines := range cases {
		dir := manifestDirOf(t)
		writeManifestSeg(t, dir, 1, good) // a healthy earlier segment must not rescue the store
		writeManifestSeg(t, dir, 2, lines...)
		set, _, err := load(t, dir, nowT)
		if err == nil || set != nil {
			t.Errorf("%s: set=%v err=%v", name, set, err)
		}
	}
	// A segment without its final newline.
	dir := manifestDirOf(t)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	writeFile(t, dir, segmentName(1, "00000000000000aa"), []byte(good))
	if _, _, err := load(t, dir, nowT); err == nil || !strings.Contains(err.Error(), "final newline") {
		t.Errorf("missing newline: %v", err)
	}
}

func TestReaderSegmentAndStoreCaps(t *testing.T) {
	dir := manifestDirOf(t)
	a, b := mfLine(t, mf(1, "1", t0)), mfLine(t, mf(2, "2", t0))
	p1 := writeManifestSeg(t, dir, 1, a)
	writeManifestSeg(t, dir, 2, b, b[:0]+a) // second segment: two lines
	st1, _ := os.Stat(p1)
	segSize := st1.Size()

	// A segment over the segment cap is dropped whole and reported.
	lim := manifestLimits{line: MaxManifestLineBytes, segment: segSize, store: 1 << 30}
	es, rep, err := loadManifestDir(dir, lim)
	if err != nil {
		t.Fatal(err)
	}
	if len(rep.Dropped) != 1 || rep.Dropped[0].Line != 0 || !strings.Contains(rep.Dropped[0].Reason, "segment is") {
		t.Errorf("dropped %+v", rep.Dropped)
	}
	if len(es) != 1 || rep.Read != 1 || rep.Segments != 2 {
		t.Errorf("entries %d, report %+v", len(es), rep)
	}
	// Exactly at the cap is accepted.
	lim.segment = segSize * 2
	if _, rep, err := loadManifestDir(dir, lim); err != nil || len(rep.Dropped) != 0 {
		t.Errorf("at cap: %v %+v", err, rep)
	}
	// A store over its cap is unusable, including bytes of dropped segments.
	lim = manifestLimits{line: MaxManifestLineBytes, segment: segSize, store: segSize * 2}
	if es, _, err := loadManifestDir(dir, lim); err == nil || es != nil {
		t.Errorf("store over cap: %d %v", len(es), err)
	}
	lim.store = segSize * 4
	if _, _, err := loadManifestDir(dir, lim); err != nil {
		t.Errorf("store under cap: %v", err)
	}
	// The line cap is injectable too.
	lim = manifestLimits{line: 10, segment: 1 << 20, store: 1 << 30}
	if _, rep, err := loadManifestDir(dir, lim); err != nil || len(rep.Dropped) != 3 {
		t.Errorf("line cap: %v %+v", err, rep)
	}
}

func TestReaderFilesystemRefusals(t *testing.T) {
	good := mfLine(t, mf(1, "1", t0))
	// Missing directory: an empty store.
	set, rep, err := load(t, manifestDirOf(t), nowT)
	if err != nil || len(set.byWeak) != 0 || rep.Segments != 0 {
		t.Errorf("missing dir: %v %v %+v", set, err, rep)
	}
	// Symlinked segment.
	dir := manifestDirOf(t)
	target := writeManifestSeg(t, filepath.Join(t.TempDir(), "x"), 1, good)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(target, filepath.Join(dir, segmentName(1, "00000000000000bb"))); err != nil {
		t.Skip("symlinks unavailable:", err)
	}
	if _, _, err := load(t, dir, nowT); err == nil {
		t.Error("symlinked segment accepted")
	}
	// Directory named like a segment.
	dir = manifestDirOf(t)
	if err := os.MkdirAll(filepath.Join(dir, segmentName(1, "00000000000000cc")), 0o755); err != nil {
		t.Fatal(err)
	}
	if _, _, err := load(t, dir, nowT); err == nil {
		t.Error("directory segment accepted")
	}
	// Non-segment names are ignored.
	dir = manifestDirOf(t)
	writeManifestSeg(t, dir, 1, good)
	writeFile(t, dir, ".tmp-123", []byte("garbage"))
	writeFile(t, dir, "README", []byte("garbage"))
	if set, _, err := load(t, dir, nowT); err != nil || len(set.Manifests(key(1))) != 1 {
		t.Errorf("stray files: %v", err)
	}
	// readManifestFile on a missing path, and on an unreadable directory path.
	if _, _, _, err := readManifestFile(filepath.Join(dir, "nope"), defaultManifestLimits); err == nil {
		t.Error("missing file")
	}
	// A dir that is a file: listSegments error.
	f := writeFile(t, t.TempDir(), "afile", []byte("x"))
	if _, _, err := OpenManifests(f, Shared).Load(nowT); err == nil {
		t.Error("ledger dir that is a file")
	}
}

func TestRetentionDistinctIdsAndK(t *testing.T) {
	dir := manifestDirOf(t)
	e := func(i int) []string { return []string{fmt.Sprintf("path:file%d", i)} }
	var lines []string
	// Ten scheduled re-records of read-set 0 (newest runs), plus read-sets 1..5
	// recorded earlier.  Re-records must not crowd out the others.
	for i := 1; i <= 5; i++ {
		lines = append(lines, mfLine(t, mf(1, fmt.Sprint(100+i), t0, e(i)...)))
	}
	for r := 0; r < 10; r++ {
		lines = append(lines, mfLine(t, mf(1, fmt.Sprint(200+r), t0, e(0)...)))
	}
	writeManifestSeg(t, dir, 1, lines...)
	set, rep, err := load(t, dir, nowT)
	if err != nil {
		t.Fatal(err)
	}
	got := set.Manifests(key(1))
	if len(got) != ManifestKeep {
		t.Fatalf("kept %d, want %d", len(got), ManifestKeep)
	}
	// Newest first: id0 (run 209), then read-sets 5, 4, 3 (runs 105, 104, 103).
	wantEntries := [][]string{e(0), e(5), e(4), e(3)}
	wantRuns := []string{"209", "105", "104", "103"}
	ids := map[[32]byte]bool{}
	for i, m := range got {
		if !reflect.DeepEqual(m.Entries, wantEntries[i]) || m.Run != wantRuns[i] {
			t.Errorf("slot %d: run %s entries %v", i, m.Run, m.Entries)
		}
		ids[m.ID()] = true
	}
	if len(ids) != ManifestKeep {
		t.Error("ids are not distinct")
	}
	if rep.Read != 15 || rep.Retained != 4 || rep.Superseded != 11 || rep.Expired != 0 {
		t.Errorf("report %+v", rep)
	}
	// Fewer distinct ids than K: all of them, no padding with duplicates.
	dir2 := manifestDirOf(t)
	writeManifestSeg(t, dir2, 1, mfLine(t, mf(1, "1", t0, e(0)...)), mfLine(t, mf(1, "2", t0, e(0)...)), mfLine(t, mf(1, "3", t0, e(1)...)))
	set2, _, _ := load(t, dir2, nowT)
	if g := set2.Manifests(key(1)); len(g) != 2 || g[0].Run != "3" || g[1].Run != "2" {
		t.Errorf("distinct: %+v", g)
	}
	// Per weak key, independently.
	dir3 := manifestDirOf(t)
	writeManifestSeg(t, dir3, 1, mfLine(t, mf(1, "1", t0)), mfLine(t, mf(2, "1", t0)))
	set3, _, _ := load(t, dir3, nowT)
	if len(set3.Manifests(key(1))) != 1 || len(set3.Manifests(key(2))) != 1 || set3.Manifests(key(3)) != nil {
		t.Error("weak isolation")
	}
}

func TestManifestsReturnsCopies(t *testing.T) {
	dir := manifestDirOf(t)
	writeManifestSeg(t, dir, 1, mfLine(t, mf(1, "1", t0)))
	set, _, _ := load(t, dir, nowT)
	a := set.Manifests(key(1))
	a[0].Entries[0] = "pkg:tampered"
	a[0].Target = "tampered"
	if b := set.Manifests(key(1)); b[0].Entries[0] == "pkg:tampered" || b[0].Target == "tampered" {
		t.Error("caller mutated the set")
	}
}

func TestRetentionExpiry(t *testing.T) {
	dir := manifestDirOf(t)
	now := t0.Add(100 * 24 * time.Hour)
	cutoff := now.Add(-ManifestMaxAge)
	writeManifestSeg(t, dir, 1,
		mfLine(t, mf(1, "1", cutoff.Add(-time.Second), "path:old")),  // expired
		mfLine(t, mf(1, "2", cutoff, "path:edge")),                   // exactly 30 days: kept
		mfLine(t, mf(1, "3", cutoff.Add(time.Second), "path:fresh")), // kept
		mfLine(t, mf(2, "1", cutoff.Add(-time.Hour), "path:old")),    // whole key expires
	)
	set, rep, err := load(t, dir, now)
	if err != nil {
		t.Fatal(err)
	}
	if rep.Expired != 2 || rep.Retained != 2 {
		t.Errorf("report %+v", rep)
	}
	if g := set.Manifests(key(1)); len(g) != 2 || g[0].Entries[0] != "path:fresh" || g[1].Entries[0] != "path:edge" {
		t.Errorf("kept %+v", g)
	}
	if set.Manifests(key(2)) != nil {
		t.Error("expired key served")
	}
	// An expired newer-id never displaces: expiry happens before the K cut.
	dir2 := manifestDirOf(t)
	var ls []string
	for i := 0; i < 6; i++ {
		ls = append(ls, mfLine(t, mf(1, fmt.Sprint(i), now.Add(-time.Hour), fmt.Sprintf("path:k%d", i))))
	}
	writeManifestSeg(t, dir2, 1, ls...)
	set2, _, _ := load(t, dir2, now.Add(ManifestMaxAge)) // now everything is just past 30 days
	if set2.Manifests(key(1)) != nil {
		t.Error("expired records served")
	}
}

func TestOrderingRunThenAtThenPosition(t *testing.T) {
	// runLess: numeric as numbers, numeric before non-numeric.
	for _, c := range []struct {
		a, b string
		less bool
	}{
		{"9", "10", true}, {"10", "9", false}, {"007", "8", true}, {"1", "1", false},
		{"0", "00", true}, {"00", "0", false}, {"99999999999999999999", "100000000000000000000", true},
		{"10", "abc", true}, {"abc", "10", false}, {"", "0", false}, {"0", "", true},
		{"a", "b", true}, {"b", "a", false}, {"a1", "a1", false},
	} {
		if runLess(c.a, c.b) != c.less {
			t.Errorf("runLess(%q,%q) != %v", c.a, c.b, c.less)
		}
	}
	dir := manifestDirOf(t)
	e := func(s string) []string { return []string{"path:" + s} }
	writeManifestSeg(t, dir, 1,
		mfLine(t, mf(1, "9", t0.Add(5*time.Hour), e("run9-late")...)),
		mfLine(t, mf(1, "10", t0, e("run10-early")...)),
		mfLine(t, mf(1, "10", t0.Add(time.Hour), e("run10-late")...)),
		mfLine(t, mf(1, "10", t0.Add(time.Hour), e("tie-line1")...)),
		mfLine(t, mf(1, "10", t0.Add(time.Hour), e("tie-line2")...)),
	)
	writeManifestSeg(t, dir, 2, mfLine(t, mf(1, "10", t0.Add(time.Hour), e("tie-seg2")...)))
	set, _, _ := load(t, dir, nowT)
	var order []string
	for _, m := range set.Manifests(key(1)) {
		order = append(order, strings.TrimPrefix(m.Entries[0], "path:"))
	}
	// Run decides first (10 beats 9 despite 9's later at), then at, then
	// segment, then line.  Only K=4 survive.
	want := []string{"tie-seg2", "tie-line2", "tie-line1", "run10-late"}
	if !reflect.DeepEqual(order, want) {
		t.Errorf("order %v want %v", order, want)
	}
	// mEntry.newer name tie-break.
	a := mEntry{m: mf(1, "1", t0), seq: 1, name: "b", line: 1}
	b := mEntry{m: mf(1, "1", t0), seq: 1, name: "a", line: 9}
	if !a.newer(b) || b.newer(a) {
		t.Error("name tie-break")
	}
}

func TestRecordAuthorityGate(t *testing.T) {
	local := fakeAuth{local: true}
	cases := []struct {
		scope Scope
		auth  Authority
		ok    bool
	}{
		{Shared, trusted, true}, {Shared, local, false}, {Shared, fakeAuth{}, false}, {Shared, nil, false},
		{Local, local, true}, {Local, trusted, false}, {Local, fakeAuth{}, false}, {Local, nil, false},
		{Scope(0), trusted, false},
	}
	for i, c := range cases {
		ledgerDir := t.TempDir()
		err := OpenManifests(ledgerDir, c.scope).Record([]Manifest{mf(1, "1", t0)}, c.auth, nowT)
		if c.ok != (err == nil) || (!c.ok && !errors.Is(err, ErrUntrusted)) {
			t.Errorf("case %d: ok=%v err=%v", i, c.ok, err)
		}
		if ents, _ := os.ReadDir(ledgerDir); !c.ok && len(ents) != 0 {
			t.Errorf("case %d: refused write left files: %v", i, ents)
		}
	}
	// Merge needs Trusted() too.
	for _, a := range []Authority{nil, fakeAuth{}, local} {
		_, err := MergeManifests(t.TempDir(), "unused", filepath.Join(t.TempDir(), "o"), a, nowT)
		if !errors.Is(err, ErrUntrusted) {
			t.Errorf("merge with %v: %v", a, err)
		}
	}
}

func TestRecordWritesImmutableSegmentsNotRestamped(t *testing.T) {
	ledgerDir := t.TempDir()
	s := OpenManifests(ledgerDir, Shared)
	if err := s.Record(nil, trusted, nowT); err != nil { // empty batch: no-op
		t.Fatal(err)
	}
	if _, err := os.Stat(filepath.Join(ledgerDir, ManifestsDir)); err == nil {
		t.Error("empty batch created the directory")
	}
	// A record dated in the past is stored exactly as given.
	old := mf(1, "7", t0.Add(-48*time.Hour), "path:a")
	if err := s.Record([]Manifest{old, mf(2, "8", t0, "path:b")}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	if err := s.Record([]Manifest{mf(1, "9", t0, "path:c")}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	segs, err := listSegments(filepath.Join(ledgerDir, ManifestsDir))
	if err != nil || len(segs) != 2 || segs[0].seq != 1 || segs[1].seq != 2 {
		t.Fatalf("segments %+v %v", segs, err)
	}
	set, _, err := s.Load(nowT)
	if err != nil {
		t.Fatal(err)
	}
	g := set.Manifests(key(1))
	if len(g) != 2 || !g[1].At.Equal(old.At) || g[0].Run != "9" {
		t.Errorf("stored %+v", g)
	}
	// Future-dated beyond the skew is refused; within it is fine.
	if err := s.Record([]Manifest{mf(1, "1", nowT.Add(MaxRecordSkew+time.Second))}, trusted, nowT); err == nil {
		t.Error("future manifest accepted")
	}
	if err := s.Record([]Manifest{mf(1, "1", nowT.Add(MaxRecordSkew))}, trusted, nowT); err != nil {
		t.Errorf("within skew refused: %v", err)
	}
	// A bad record refuses the batch whole.
	bad := mf(1, "1", t0)
	bad.Target = ""
	before, _ := listSegments(filepath.Join(ledgerDir, ManifestsDir))
	if err := s.Record([]Manifest{mf(3, "1", t0), bad}, trusted, nowT); err == nil {
		t.Error("bad batch accepted")
	}
	if after, _ := listSegments(filepath.Join(ledgerDir, ManifestsDir)); len(after) != len(before) {
		t.Error("bad batch wrote a segment")
	}
}

func TestManifestRecordErrors(t *testing.T) {
	// ManifestsDir exists as a file: MkdirAll fails.
	ledgerDir := t.TempDir()
	writeFile(t, ledgerDir, ManifestsDir, []byte("x"))
	if err := OpenManifests(ledgerDir, Shared).Record([]Manifest{mf(1, "1", t0)}, trusted, nowT); err == nil {
		t.Error("MkdirAll failure swallowed")
	}
	// Injected write failure surfaces and leaves nothing.
	dir := t.TempDir()
	s := OpenManifests(dir, Shared)
	s.fs = fsOps{
		create: func(d string) (tmpFile, error) {
			f, err := os.CreateTemp(d, ".tmp-*")
			return &fakeTmp{File: f, failWrite: true}, err
		},
		rename: os.Rename,
	}
	if err := s.Record([]Manifest{mf(1, "1", t0)}, trusted, nowT); err == nil {
		t.Error("write failure swallowed")
	}
	if ents, _ := os.ReadDir(filepath.Join(dir, ManifestsDir)); len(ents) != 0 {
		t.Errorf("left files: %v", ents)
	}
	// A corrupt directory refuses a new record rather than numbering past it.
	d2 := t.TempDir()
	if err := os.MkdirAll(filepath.Join(d2, ManifestsDir, segmentName(1, "00000000000000dd")), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := OpenManifests(d2, Shared).Record([]Manifest{mf(1, "1", t0)}, trusted, nowT); err == nil {
		t.Error("record into a corrupt directory")
	}
	// Segment cap on a batch, via packLines.
	if _, err := packLines([][]byte{make([]byte, 11)}, 10); err == nil {
		t.Error("oversize line packed")
	}
	segs, err := packLines([][]byte{make([]byte, 6), make([]byte, 6), make([]byte, 4)}, 10)
	if err != nil || len(segs) != 2 || len(segs[0]) != 6 || len(segs[1]) != 10 {
		t.Errorf("pack: %v %d", err, len(segs))
	}
	if segs, err := packLines(nil, 10); err != nil || segs != nil {
		t.Errorf("pack nothing: %v %v", segs, err)
	}
}

func TestRecordBatchTooBigForOneSegment(t *testing.T) {
	// Each line stays under the 1 MiB line cap; 70 of them exceed 64 MiB.
	if testing.Short() {
		t.Skip("allocates tens of MiB")
	}
	m := mf(1, "1", t0)
	m.Entries = manyEntries(MaxManifestEntries, 110) // about 1.0 MB, under the line cap
	line, err := m.Encode()
	if err != nil {
		t.Fatal(err)
	}
	n := MaxManifestSegmentBytes/len(line) + 1
	batch := make([]Manifest, n)
	for i := range batch {
		batch[i] = m
	}
	err = OpenManifests(t.TempDir(), Shared).Record(batch, trusted, nowT)
	if err == nil || !strings.Contains(err.Error(), "segment limit") {
		t.Errorf("err = %v", err)
	}
}

// ---------- merge ----------

func snapshotOf(t *testing.T, ms ...Manifest) string {
	t.Helper()
	ledgerDir := t.TempDir()
	if len(ms) > 0 {
		if err := OpenManifests(ledgerDir, Shared).Record(ms, trusted, nowT); err != nil {
			t.Fatal(err)
		}
	}
	return filepath.Join(ledgerDir, ManifestsDir)
}

func deltaOf(t *testing.T, ms ...Manifest) string {
	t.Helper()
	var b []byte
	for _, m := range ms {
		l, err := m.Encode()
		if err != nil {
			t.Fatal(err)
		}
		b = append(b, l...)
	}
	return writeFile(t, t.TempDir(), "delta.jsonl", b)
}

func readDirFiles(t *testing.T, dir string) map[string]string {
	t.Helper()
	ents, err := os.ReadDir(dir)
	if err != nil {
		t.Fatal(err)
	}
	out := map[string]string{}
	for _, e := range ents {
		b, _ := os.ReadFile(filepath.Join(dir, e.Name()))
		out[e.Name()] = string(b)
	}
	return out
}

func TestMergeManifestsUnionRetentionAndDeterminism(t *testing.T) {
	snapA := mf(1, "10", t0, "path:a")
	snapB := mf(2, "10", t0, "path:b")
	snapOld := mf(3, "10", nowT.Add(-ManifestMaxAge-time.Hour), "path:ancient")
	d1 := mf(1, "11", t0.Add(time.Minute), "path:c")
	d2 := mf(1, "12", t0.Add(2*time.Minute), "path:a") // re-record of snapA's id
	run := func(snapshot []Manifest, delta []Manifest) (string, map[string]string, ManifestMergeStats) {
		out := filepath.Join(t.TempDir(), "out")
		st, err := MergeManifests(snapshotOf(t, snapshot...), deltaOf(t, delta...), out, trusted, nowT)
		if err != nil {
			t.Fatal(err)
		}
		return out, readDirFiles(t, out), st
	}
	out, files, st := run([]Manifest{snapA, snapB, snapOld}, []Manifest{d1, d2})
	if st.Snapshot != 3 || st.Delta != 2 || st.Expired != 1 || st.Superseded != 1 || st.Kept != 3 {
		t.Errorf("stats %+v", st)
	}
	es, _, err := loadManifestDir(out, defaultManifestLimits)
	if err != nil {
		t.Fatal(err)
	}
	got := retainManifests(es, nowT, &ManifestReport{})
	w1 := got[key(1)]
	if len(w1) != 2 || w1[0].m.Run != "12" || w1[1].m.Run != "11" {
		t.Errorf("weak 1: %+v", w1)
	}
	if len(got[key(2)]) != 1 || len(got[key(3)]) != 0 {
		t.Error("weak 2 / 3")
	}
	// Deterministic: same inputs, same bytes (names included), whatever the
	// order the delta lines arrive in.
	_, files2, _ := run([]Manifest{snapA, snapB, snapOld}, []Manifest{d1, d2})
	if !reflect.DeepEqual(files, files2) {
		t.Error("merge is not byte-deterministic")
	}
	_, files3, _ := run([]Manifest{snapOld, snapB, snapA}, []Manifest{d2, d1})
	if len(files3) != len(files) {
		t.Fatal("segment count differs")
	}
	var c1, c2 []string
	for _, v := range files {
		c1 = append(c1, v)
	}
	for _, v := range files3 {
		c2 = append(c2, v)
	}
	if !reflect.DeepEqual(c1, c2) {
		t.Errorf("reordering inputs changed the content\n%v\n%v", c1, c2)
	}
	// Idempotent: merging the result with an empty delta reproduces it.
	out2 := filepath.Join(t.TempDir(), "again")
	if _, err := MergeManifests(out, deltaOf(t), out2, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	again := readDirFiles(t, out2)
	if len(again) != 1 || len(files) != 1 {
		t.Fatalf("segments %d %d", len(again), len(files))
	}
	for _, v := range again {
		for _, w := range files {
			if v != w {
				t.Error("re-merge changed the content")
			}
		}
	}
	// Segment numbering continues the snapshot's (out is seq 2, this is 3).
	for name := range again {
		if seq, ok := parseSegmentName(name); !ok || seq != 3 {
			t.Errorf("name %s", name)
		}
	}
	// No partial directory is left behind.
	ents, _ := os.ReadDir(filepath.Dir(out))
	for _, e := range ents {
		if strings.Contains(e.Name(), ".partial-") {
			t.Errorf("leftover %s", e.Name())
		}
	}
}

func TestMergeKeepsTiesStableAcrossCompaction(t *testing.T) {
	// Two ids for one weak key with identical (run, at): file position decides.
	// After a merge, the next merge must rank them the same way.
	a := mf(1, "5", t0, "path:first")
	b := mf(1, "5", t0, "path:second")
	snap := snapshotOf(t, a, b)
	out1 := filepath.Join(t.TempDir(), "o1")
	if _, err := MergeManifests(snap, deltaOf(t), out1, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	es1, _, _ := loadManifestDir(snap, defaultManifestLimits)
	es2, _, _ := loadManifestDir(out1, defaultManifestLimits)
	r1 := retainManifests(es1, nowT, &ManifestReport{})[key(1)]
	r2 := retainManifests(es2, nowT, &ManifestReport{})[key(1)]
	if len(r1) != 2 || len(r2) != 2 || r1[0].m.Entries[0] != r2[0].m.Entries[0] || r1[0].m.Entries[0] != "path:second" {
		t.Errorf("tie order changed: %v vs %v", r1[0].m.Entries, r2[0].m.Entries)
	}
}

func TestMergeManifestsRefusals(t *testing.T) {
	good := mf(1, "1", t0)
	snap := snapshotOf(t, good)
	out := func() string { return filepath.Join(t.TempDir(), "out") }
	absent := func(p string) {
		t.Helper()
		if _, err := os.Stat(p); err == nil {
			t.Errorf("%s exists after a refused merge", p)
		}
	}

	// A bad delta rejects the whole delta.
	bad := mfLine(t, mf(2, "1", t0))
	badDelta := writeFile(t, t.TempDir(), "d.jsonl", []byte(mfLine(t, good)+"\n"+strings.Replace(bad, `,"os"`, `, "os"`, 1)+"\n"))
	o := out()
	if _, err := MergeManifests(snap, badDelta, o, trusted, nowT); err == nil || !strings.Contains(err.Error(), "delta") {
		t.Errorf("bad delta: %v", err)
	}
	absent(o)
	// An over-cap record in the delta also rejects it (the writer must refuse to emit one).
	tooMany := mf(2, "1", t0)
	tooMany.Entries = manyEntries(MaxManifestEntries+1, 6)
	capDelta := writeFile(t, t.TempDir(), "d.jsonl", []byte(overCapLine(tooMany)+"\n"))
	if _, err := MergeManifests(snap, capDelta, out(), trusted, nowT); err == nil || !strings.Contains(err.Error(), "delta") {
		t.Errorf("over-cap delta: %v", err)
	}
	// A far-future delta record.
	if _, err := MergeManifests(snap, deltaOf(t, mf(2, "1", nowT.Add(MaxClockSkew+time.Second))), out(), trusted, nowT); err == nil {
		t.Error("future delta accepted")
	}
	// Missing delta, oversize delta.
	if _, err := MergeManifests(snap, filepath.Join(t.TempDir(), "missing"), out(), trusted, nowT); err == nil {
		t.Error("missing delta")
	}
	big := writeFile(t, t.TempDir(), "d.jsonl", []byte(mfLine(t, good)+"\n"))
	if _, err := mergeManifests(realFS, manifestLimits{MaxManifestLineBytes, 10, 1 << 30}, snap, big, out(), trusted, nowT); err == nil ||
		!strings.Contains(err.Error(), "larger than") {
		t.Errorf("oversize delta: %v", err)
	}
	// Corrupt snapshot fails the merge.
	corrupt := manifestDirOf(t)
	writeManifestSeg(t, corrupt, 1, "garbage")
	if _, err := MergeManifests(corrupt, deltaOf(t), out(), trusted, nowT); err == nil || !strings.Contains(err.Error(), "snapshot") {
		t.Errorf("corrupt snapshot: %v", err)
	}
	// Output directory rules.
	if _, err := MergeManifests(snap, deltaOf(t), snap, trusted, nowT); err == nil {
		t.Error("out == snapshot")
	}
	full := t.TempDir()
	writeFile(t, full, "x", []byte("x"))
	if _, err := MergeManifests(snap, deltaOf(t), full, trusted, nowT); err == nil {
		t.Error("non-empty out")
	}
	// Bad now.
	if _, err := MergeManifests(snap, deltaOf(t), out(), trusted, time.Time{}); err == nil {
		t.Error("zero now")
	}
	// Over-cap snapshot records are dropped, reported, and not carried over.
	dropDir := manifestDirOf(t)
	writeManifestSeg(t, dropDir, 1, mfLine(t, good), overCapLine(tooMany))
	o = out()
	st, err := MergeManifests(dropDir, deltaOf(t), o, trusted, nowT)
	if err != nil || len(st.Dropped) != 1 || st.Kept != 1 || st.Snapshot != 1 {
		t.Errorf("snapshot drops: %v %+v", err, st)
	}
	// A merge into an empty (existing) directory works.
	empty := filepath.Join(t.TempDir(), "empty")
	if err := os.Mkdir(empty, 0o755); err != nil {
		t.Fatal(err)
	}
	if _, err := MergeManifests(snap, deltaOf(t), empty, trusted, nowT); err != nil {
		t.Errorf("empty out dir: %v", err)
	}
	// Merging nothing with nothing writes an empty snapshot directory.
	o = out()
	if st, err := MergeManifests(snapshotOf(t), deltaOf(t), o, trusted, nowT); err != nil || st.Kept != 0 {
		t.Errorf("empty merge: %v %+v", err, st)
	}
	if ents, err := os.ReadDir(o); err != nil || len(ents) != 0 {
		t.Errorf("empty merge output: %v %v", ents, err)
	}
}

func TestMergeManifestsSplitsSegmentsAtTheCap(t *testing.T) {
	var ms []Manifest
	for i := 0; i < 5; i++ {
		ms = append(ms, mf(byte(i+1), "1", t0))
	}
	line, _ := ms[0].Encode()
	// The segment cap fits 2 lines; the delta file (5 lines) is itself over
	// that cap, so a one-segment-per-merge limit is exercised with a delta of
	// 2 lines applied to a snapshot of 3.
	lim := manifestLimits{MaxManifestLineBytes, int64(len(line)) * 3, 1 << 30}
	snap := manifestDirOf(t)
	writeManifestSeg(t, snap, 1, mfLine(t, ms[0]), mfLine(t, ms[1]), mfLine(t, ms[2]))
	o := filepath.Join(t.TempDir(), "o")
	if _, err := mergeManifests(realFS, lim, snap, deltaOf(t, ms[3], ms[4]), o, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	files := readDirFiles(t, o)
	if len(files) != 2 {
		t.Fatalf("want 2 segments, got %d", len(files))
	}
	total := 0
	for name, data := range files {
		if int64(len(data)) > lim.segment {
			t.Errorf("%s is %d bytes, over the cap", name, len(data))
		}
		total += strings.Count(data, "\n")
	}
	if total != 5 {
		t.Errorf("records %d", total)
	}
}

func TestMergeManifestsFailureInjection(t *testing.T) {
	snap := snapshotOf(t, mf(1, "1", t0))
	delta := deltaOf(t, mf(2, "2", t0))
	cases := map[string]fsOps{
		"write": {create: func(d string) (tmpFile, error) {
			f, err := os.CreateTemp(d, ".tmp-*")
			return &fakeTmp{File: f, failWrite: true}, err
		}, rename: os.Rename},
		"create": {create: func(string) (tmpFile, error) { return nil, errors.New("no temp") }},
		"rename": {create: realFS.create, syncDir: realFS.syncDir,
			rename: func(string, string) error { return errors.New("rename failed") }},
		"syncdir": {create: realFS.create, rename: os.Rename,
			syncDir: func(string) error { return errors.New("sync failed") }},
	}
	for name, ops := range cases {
		o := filepath.Join(t.TempDir(), "out")
		if _, err := mergeManifests(ops, defaultManifestLimits, snap, delta, o, trusted, nowT); err == nil {
			t.Errorf("%s: no error", name)
		}
		if name != "syncdir" {
			if _, err := os.Stat(o); err == nil {
				t.Errorf("%s: output exists", name)
			}
			ents, _ := os.ReadDir(filepath.Dir(o))
			for _, e := range ents {
				if strings.Contains(e.Name(), ".partial-") {
					t.Errorf("%s: partial directory left behind", name)
				}
			}
		}
	}
	// The partial directory cannot be created: its parent is a file.
	parent := writeFile(t, t.TempDir(), "afile", []byte("x"))
	if _, err := MergeManifests(snap, delta, filepath.Join(parent, "out"), trusted, nowT); err == nil {
		t.Error("unwritable output parent")
	}
	// Segment sequence exhausted.
	exh := manifestDirOf(t)
	if err := os.MkdirAll(exh, 0o755); err != nil {
		t.Fatal(err)
	}
	writeFile(t, exh, segmentName(maxSeq, "00000000000000ee"), []byte(mfLine(t, mf(1, "1", t0))+"\n"))
	if _, err := MergeManifests(exh, delta, filepath.Join(t.TempDir(), "o"), trusted, nowT); err == nil ||
		!strings.Contains(err.Error(), "exhausted") {
		t.Errorf("exhausted: %v", err)
	}
	if err := OpenManifests(filepath.Dir(exh), Shared).Record([]Manifest{mf(1, "1", t0)}, trusted, nowT); err == nil ||
		!strings.Contains(err.Error(), "exhausted") {
		t.Errorf("record exhausted: %v", err)
	}
}

// ---------- isolation from the pass ledger ----------

func TestPassLedgerIgnoresManifestsSubdirectory(t *testing.T) {
	ledgerDir := t.TempDir()
	// A real pass in the ledger, a real manifest in its manifests/ subdirectory.
	if err := Open(ledgerDir, Shared).Record([]Record{pass(1, t0)}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	if err := OpenManifests(ledgerDir, Shared).Record([]Manifest{mf(9, "1", t0)}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	got, err := Open(ledgerDir, Shared).Lookup([][32]byte{key(1), key(9)})
	if err != nil || len(got) != 1 {
		t.Fatalf("lookup: %v %v", got, err)
	}
	if _, ok := got[key(1)]; !ok {
		t.Fatal("pass lost")
	}
	if _, ok := got[key(9)]; ok {
		t.Error("a manifest weak key produced a pass hit")
	}
	// The directory listing the pass reader uses does not see the subdirectory
	// (its name is not a segment name), so nothing in manifests/ is opened.
	segs, err := listSegments(ledgerDir)
	if err != nil || len(segs) != 1 {
		t.Errorf("pass segments %+v %v", segs, err)
	}
	// Even corrupt manifests cannot disturb pass lookups...
	writeFile(t, filepath.Join(ledgerDir, ManifestsDir), segmentName(50, "00000000000000ff"), []byte("garbage\n"))
	if got, err := Open(ledgerDir, Shared).Lookup([][32]byte{key(1)}); err != nil || len(got) != 1 {
		t.Errorf("corrupt manifests disturbed the pass ledger: %v %v", got, err)
	}
	// ...and the pass merge ignores them too.
	out := filepath.Join(t.TempDir(), "o")
	delta := writeFile(t, t.TempDir(), "d.jsonl", mustEncode(t, pass(2, t0)))
	if _, err := Merge(ledgerDir, delta, out, trusted, nowT, nil); err != nil {
		t.Errorf("pass merge: %v", err)
	}
	if _, err := os.Stat(filepath.Join(out, ManifestsDir)); err == nil {
		t.Error("pass merge copied manifests")
	}
	// A directory named "manifests" is not a segment even though ledgerDir
	// holds it: the name pattern is what makes this true.
	if _, ok := parseSegmentName(ManifestsDir); ok {
		t.Error("'manifests' parses as a segment name")
	}
	// And the manifest reader never looks at the pass segments beside it.
	if _, _, err := OpenManifests(ledgerDir, Shared).Load(nowT); err == nil {
		t.Error("the garbage manifest segment should make the manifest store unusable")
	}
	clean := t.TempDir()
	if err := Open(clean, Shared).Record([]Record{pass(1, t0)}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	set, rep, err := OpenManifests(clean, Shared).Load(nowT)
	if err != nil || rep.Segments != 0 || set.Manifests(key(1)) != nil {
		t.Errorf("manifest reader saw pass segments: %v %+v", err, rep)
	}
}

func mustEncode(t *testing.T, r Record) []byte {
	t.Helper()
	b, err := r.Encode()
	if err != nil {
		t.Fatal(err)
	}
	return b
}

func TestManifestFileInPassDirectoryIsRefused(t *testing.T) {
	ledgerDir := t.TempDir()
	if err := Open(ledgerDir, Shared).Record([]Record{pass(1, t0)}, trusted, nowT); err != nil {
		t.Fatal(err)
	}
	// A well-formed manifest segment dropped among the pass segments.
	mlines, _ := mf(2, "1", t0).Encode()
	writeFile(t, ledgerDir, segmentName(2, "0123456789abcdef"), mlines)
	if got, err := Open(ledgerDir, Shared).Lookup([][32]byte{key(1)}); err == nil || got != nil {
		t.Errorf("pass reader accepted a manifest segment: %v %v", got, err)
	}
	if _, err := ParseSegment(bytes.NewReader(mlines)); err == nil {
		t.Error("ParseSegment accepted a manifest line")
	}
	out := filepath.Join(t.TempDir(), "o")
	if _, err := Merge(ledgerDir, writeFile(t, t.TempDir(), "d.jsonl", mlines), out, trusted, nowT, nil); err == nil {
		t.Error("pass merge accepted a manifest delta")
	}
	// The mirror image: a pass segment dropped into manifests/ makes the
	// manifest store unusable.
	pl, _ := pass(3, t0).Encode()
	dir := manifestDirOf(t)
	writeManifestSeg(t, dir, 1, mfLine(t, mf(1, "1", t0)))
	writeFile(t, dir, segmentName(2, "0123456789abcdef"), pl)
	if set, _, err := load(t, dir, nowT); err == nil || set != nil {
		t.Errorf("manifest reader accepted a pass segment: %v %v", set, err)
	}
}

func TestShort40TruncatesOnRuneBoundary(t *testing.T) {
	if short40("abc") != "abc" {
		t.Error("short strings pass through")
	}
	got := short40("x" + strings.Repeat("é", 30)) // byte 40 falls mid-rune
	if !strings.HasSuffix(got, "...") || strings.ContainsRune(got, '�') {
		t.Errorf("bad truncation %q", got)
	}
	// And a long bad entry yields a bounded error message.
	err := mf(1, "1", t0, "env:"+strings.Repeat("é", 200)).Validate()
	if err == nil || len(err.Error()) > 200 {
		t.Errorf("error not bounded: %v", err)
	}
}

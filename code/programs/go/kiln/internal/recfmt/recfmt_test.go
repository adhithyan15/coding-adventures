package recfmt

import (
	"bytes"
	"encoding/json"
	"errors"
	"io"
	"strings"
	"testing"
	"testing/iotest"
)

const (
	d1 = "1111111111111111111111111111111111111111111111111111111111111111"
	d2 = "2222222222222222222222222222222222222222222222222222222222222222"
)

func goodBinding() Binding {
	return Binding{
		Schema: SchemaBinding, Repository: "owner/repo", Run: "43117337685", Attempt: 1,
		Revision: strings.Repeat("a", 40), ShardIndex: 3, ShardCount: 12,
		GraphSHA256: d1, StepsSHA256: d1, PartitionSHA256: d1, HostFactsSHA256: d1, InventorySHA256: d1,
		SandboxConfigSHA256: d1, KilnSHA256: d1, BuildToolSHA256: d1, StraceSHA256: d1, GitSHA256: d1, Recorder: 1,
		StraceVersion: "strace -- version 6.8", Kernel: "6.8.0-1", JobStart: 1794000000,
		PreparedAt: "2026-11-13T02:03:04Z",
	}
}

func goodFacts() HostFacts {
	return HostFacts{Schema: SchemaHostFacts, Image: "ubuntu-24.04/20261101.1", Root: "/home/runner/work/r/r",
		Home: "/home/runner", Tmpdir: "/tmp/kiln-tmp", ExecutorArgs: []string{"-clippy=false"},
		Env: []string{"LANG=C.UTF-8", "TZ=UTC"}}
}

func goodRecord(action string) ShardRecord {
	return ShardRecord{
		BindingID: d1, Action: action, Language: "go", Status: StatusBuilt, StepsRun: 1, StepsTotal: 1,
		Steps: []Step{{WallMS: 5, TraceBytes: 100, Clean: true}}, StepsSHA: d2, Weak: d2,
		Record:   RawRecord{Package: action, Entries: []string{"pkg:" + action}, Writes: []string{}, WritesComplete: true},
		Complete: &complete,
	}
}

var complete = true

func mustJSON(t *testing.T, v any) []byte {
	t.Helper()
	b, err := json.Marshal(v)
	if err != nil {
		t.Fatal(err)
	}
	return b
}

func TestRoundTripAndCanonicalForm(t *testing.T) {
	b, err := EncodeBinding(goodBinding())
	if err != nil {
		t.Fatal(err)
	}
	got, err := DecodeBinding(b)
	if err != nil || got != goodBinding() {
		t.Fatalf("binding round trip: %v %+v", err, got)
	}
	f, err := EncodeHostFacts(goodFacts())
	if err != nil {
		t.Fatal(err)
	}
	gf, err := DecodeHostFacts(f)
	if err != nil || gf.ImageID() != goodFacts().Image || len(gf.Environ()) != 2 {
		t.Fatalf("facts round trip: %v %+v", err, gf)
	}
	i, err := EncodeInventory(Inventory{Schema: SchemaInventory})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := DecodeInventory(i); err != nil {
		t.Fatalf("an empty inventory must encode as [] and decode: %v", err)
	}
	h := ShardHeader{Schema: SchemaShard, BindingID: d1, ShardIndex: 3, ShardCount: 12}
	hl, err := EncodeHeader(h)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(hl), `"void":[]`) {
		t.Fatalf("nil void must encode as []: %s", hl)
	}
	if _, err := DecodeHeader(hl); err != nil {
		t.Fatal(err)
	}
	rl, err := EncodeRecord(goodRecord("code/packages/go/a"))
	if err != nil {
		t.Fatal(err)
	}
	if _, err := DecodeRecord(rl); err != nil {
		t.Fatalf("%v\n%s", err, rl)
	}
	// Encoding is deterministic.
	rl2, _ := EncodeRecord(goodRecord("code/packages/go/a"))
	if !bytes.Equal(rl, rl2) {
		t.Fatal("the encoder is not deterministic")
	}
}

func TestDecodeShardEndToEnd(t *testing.T) {
	bind, _ := EncodeBinding(goodBinding())
	facts, _ := EncodeHostFacts(goodFacts())
	inv, _ := EncodeInventory(Inventory{Schema: SchemaInventory, Paths: []string{"a/b", "c"}})
	recs, err := EncodeShardRecords(ShardHeader{Schema: SchemaShard, BindingID: d1, ShardIndex: 3, ShardCount: 12, Void: []string{"tracked-changed"}},
		[]ShardRecord{goodRecord("a"), goodRecord("b")})
	if err != nil {
		t.Fatal(err)
	}
	s, err := DecodeShard(3, ShardFiles{bind, facts, inv, bytes.NewReader(recs)}, DefaultLimits)
	if err != nil {
		t.Fatal(err)
	}
	if s.ArtifactIndex != 3 || len(s.Records) != 2 || s.Header.Void[0] != "tracked-changed" ||
		s.BindingID != sum(bind) || s.HostFactsSHA256 != sum(facts) || s.InventorySHA256 != sum(inv) {
		t.Fatalf("%+v", s)
	}
	// The trailing newline is optional.
	if _, err := DecodeShard(3, ShardFiles{bind, facts, inv, bytes.NewReader(bytes.TrimSuffix(recs, []byte("\n")))}, DefaultLimits); err != nil {
		t.Fatal(err)
	}
	// A header alone is a shard with no records.
	hdr, _, _ := bytes.Cut(recs, []byte("\n"))
	if s, err := DecodeShard(3, ShardFiles{bind, facts, inv, bytes.NewReader(hdr)}, DefaultLimits); err != nil || len(s.Records) != 0 {
		t.Fatalf("%v", err)
	}
}

func TestDecodeShardRefusals(t *testing.T) {
	bind, _ := EncodeBinding(goodBinding())
	facts, _ := EncodeHostFacts(goodFacts())
	inv, _ := EncodeInventory(Inventory{Schema: SchemaInventory})
	recs, _ := EncodeShardRecords(ShardHeader{Schema: SchemaShard, BindingID: d1, ShardIndex: 3, ShardCount: 12}, []ShardRecord{goodRecord("a")})
	ok := func() ShardFiles { return ShardFiles{bind, facts, inv, bytes.NewReader(recs)} }
	cases := map[string]func(f *ShardFiles){
		"bad binding":   func(f *ShardFiles) { f.Binding = []byte(`{}`) },
		"bad facts":     func(f *ShardFiles) { f.HostFacts = []byte(`{}`) },
		"bad inventory": func(f *ShardFiles) { f.Inventory = []byte(`{}`) },
		"no stream":     func(f *ShardFiles) { f.Records = nil },
		"empty stream":  func(f *ShardFiles) { f.Records = strings.NewReader("") },
		"bad header":    func(f *ShardFiles) { f.Records = strings.NewReader("{}\n") },
		"bad record": func(f *ShardFiles) {
			f.Records = strings.NewReader(strings.SplitN(string(recs), "\n", 2)[0] + "\n{}\n")
		},
		"empty line": func(f *ShardFiles) { f.Records = strings.NewReader(strings.SplitN(string(recs), "\n", 2)[0] + "\n\n") },
		"read error": func(f *ShardFiles) { f.Records = iotest.ErrReader(errors.New("boom")) },
	}
	for name, mut := range cases {
		f := ok()
		mut(&f)
		if _, err := DecodeShard(3, f, DefaultLimits); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
}

func TestStrictDecoding(t *testing.T) {
	line := string(mustLine(t, goodRecord("a")))
	// Replace one member of the canonical record line.
	repl := func(from, to string) string {
		if !strings.Contains(line, from) {
			t.Fatalf("fixture drifted: %q not in %s", from, line)
		}
		return strings.Replace(line, from, to, 1)
	}
	cases := map[string]string{
		"unknown key":         repl(`"language":"go"`, `"language":"go","extra":1`),
		"duplicate key":       repl(`"language":"go"`, `"language":"go","language":"go"`),
		"case-variant key":    repl(`"language"`, `"Language"`),
		"trailing data":       line + ` {}`,
		"trailing garbage":    line + `x`,
		"missing key":         repl(`"language":"go",`, ``),
		"missing nested key":  repl(`"untracked_observations":[],`, ``),
		"null list":           repl(`"untracked_observations":[]`, `"untracked_observations":null`),
		"null steps":          repl(`"steps":[{"wall_ms":5,"trace_bytes":100,"clean":true}]`, `"steps":null`),
		"null record":         repl(`"record":{`, `"record":null,"x":{`),
		"wrong type":          repl(`"steps_run":1`, `"steps_run":"1"`),
		"fractional":          repl(`"steps_run":1`, `"steps_run":1.5`),
		"missing step key":    repl(`"clean":true`, ``),
		"nested unknown key":  repl(`"clean":true`, `"clean":true,"z":0`),
		"not an object":       `[]`,
		"null":                `null`,
		"empty":               ``,
		"nested duplicate":    repl(`"writes":[]`, `"writes":[],"writes":[]`),
		"record is a string":  repl(`"record":{`, `"record":"{`),
		"upper-case digest":   repl(`"weak":"`+d2, `"weak":"`+strings.Repeat("A", 64)),
		"short digest":        repl(d1, d1[:63]),
		"weak and refusal":    repl(`"refusal":""`, `"refusal":"x"`),
		"neither":             repl(`"weak":"`+d2+`"`, `"weak":""`),
		"steps_run>total":     repl(`"steps_run":1`, `"steps_run":2`),
		"negative":            repl(`"steps_run":1`, `"steps_run":-1`),
		"built without steps": repl(`"steps_sha256":"`+d2+`"`, `"steps_sha256":""`),
		"empty status":        repl(`"status":"built"`, `"status":""`),
		"package mismatch":    repl(`"package":"a"`, `"package":"b"`),
		"control in text":     repl(`"language":"go"`, `"language":"g\no"`),
	}
	for name, in := range cases {
		if _, err := DecodeRecord([]byte(in)); err == nil {
			t.Errorf("%s: accepted: %s", name, in)
		}
	}
	// The same checks hold for the other files.
	for name, in := range map[string]string{
		"binding unknown": `{"schema":"kiln-record-binding/1","zzz":1}`,
		"binding empty":   `{}`,
		"facts dup":       `{"schema":"x","schema":"x"}`,
		"inventory case":  `{"Schema":"kiln-record-inventory/1","paths":[]}`,
	} {
		_, e1 := DecodeBinding([]byte(in))
		_, e2 := DecodeHostFacts([]byte(in))
		_, e3 := DecodeInventory([]byte(in))
		_, e4 := DecodeHeader([]byte(in))
		if e1 == nil || e2 == nil || e3 == nil || e4 == nil {
			t.Errorf("%s accepted", name)
		}
	}
	// writes_complete is REQUIRED (recorder version 2, KILN04 7.1 M3): a record
	// without it does not decode, and the writer cannot encode one.
	l := mustLine(t, goodRecord("a"))
	got, err := DecodeRecord(l)
	if err != nil || got.Complete == nil || !*got.Complete {
		t.Fatalf("writes_complete: %v %+v", err, got)
	}
	noKey := bytes.Replace(l, []byte(`,"writes_complete":true}`), []byte("}"), 1)
	if bytes.Equal(noKey, l) {
		t.Fatalf("setup: %s", l)
	}
	if _, err := DecodeRecord(noKey); err == nil {
		t.Error("a record without writes_complete decoded")
	}
	// The writer's check: present, and equal to the classifier's own answer.
	r := goodRecord("a")
	r.Complete = nil
	if _, err := EncodeRecord(r); err == nil {
		t.Error("EncodeRecord took a missing writes_complete")
	}
	if err := CheckComplete(r); err == nil {
		t.Error("CheckComplete took a missing writes_complete")
	}
	r = goodRecord("a")
	f := false
	r.Complete = &f // the classifier said true
	if _, err := EncodeRecord(r); err == nil {
		t.Error("EncodeRecord took a writes_complete the classifier did not say")
	}
	r = goodRecord("a")
	r.Record.WritesComplete = false // the classifier said false
	if err := CheckComplete(r); err == nil {
		t.Error("CheckComplete took a true the classifier did not say")
	}
	r.Complete = CompleteOf(r.Record)
	if err := CheckComplete(r); err != nil {
		t.Error(err)
	}
}

func mustLine(t *testing.T, r ShardRecord) []byte {
	t.Helper()
	l, err := EncodeRecord(r)
	if err != nil {
		t.Fatal(err)
	}
	return l
}

// Path spellings that would let an overlap check miss must not decode.
func TestPathSpellingsAreRefused(t *testing.T) {
	for _, bad := range []string{"", ".", "..", "/a", "a/", "a//b", "a/./b", "a/../b", "a/b/", "\xff"} {
		for _, field := range []string{"writes", "tracked_writes", "absent_observations", "untracked_observations", "traversed"} {
			r := goodRecord("a")
			switch field {
			case "writes":
				r.Record.Writes = []string{bad}
			case "tracked_writes":
				r.Record.TrackedWrites = []string{bad}
			case "absent_observations":
				r.Record.AbsentObservations = []string{bad}
			case "untracked_observations":
				r.Record.UntrackedObservations = []string{bad}
			default:
				r.Record.Traversed = []string{bad}
			}
			if err := r.Validate(); err == nil {
				t.Errorf("%s accepted %q", field, bad)
			}
		}
		if err := (&Inventory{Schema: SchemaInventory, Paths: []string{bad}}).Validate(); err == nil {
			t.Errorf("inventory accepted %q", bad)
		}
	}
	// Case matters: "A" and "a" are two paths, so both may be listed.
	r := goodRecord("a")
	r.Record.Writes = []string{"A", "a"}
	if err := r.Validate(); err != nil {
		t.Errorf("case-distinct paths: %v", err)
	}
	// Prefix confusion is not a format matter: a/b and a/bc are both fine.
	r.Record.Writes = []string{"a/b", "a/bc"}
	if err := r.Validate(); err != nil {
		t.Error(err)
	}
}

func TestListsMustBeSortedUniqueAndCapped(t *testing.T) {
	r := goodRecord("a")
	r.Record.Writes = []string{"b", "a"}
	if r.Validate() == nil {
		t.Error("unsorted accepted")
	}
	r.Record.Writes = []string{"a", "a"}
	if r.Validate() == nil {
		t.Error("duplicate accepted")
	}
	r = goodRecord("a")
	r.Record.Writes = make([]string, MaxList+1)
	for i := range r.Record.Writes {
		r.Record.Writes[i] = "p"
	}
	if r.Validate() == nil {
		t.Error("over-cap list accepted")
	}
	r = goodRecord("a")
	for _, mut := range []func(){
		func() { r.Record.Entries = []string{"b", "a"} },
		func() { r.Record.Binds = []string{"x", "x"} },
		func() { r.Record.Reasons = []string{"b", "a"} },
		func() { r.Record.Reasons = []string{"a\nb"} },
		func() { r.Record.Entries = []string{strings.Repeat("e", MaxText+1)} },
		func() {
			r.Record.Reasons = nil
			for i := 0; i <= MaxReasonsPerKind; i++ {
				r.Record.Reasons = append(r.Record.Reasons, "daemon:"+strings.Repeat("a", i+1))
			}
		},
	} {
		r = goodRecord("a")
		mut()
		if r.Validate() == nil {
			t.Errorf("accepted %+v", r.Record)
		}
	}
	// Sixteen of one kind is the classifier's cap and is fine.
	r = goodRecord("a")
	for i := 0; i < MaxReasonsPerKind; i++ {
		r.Record.Reasons = append(r.Record.Reasons, "daemon:"+strings.Repeat("a", i+1))
	}
	if err := r.Validate(); err != nil {
		t.Error(err)
	}
}

func TestBindingValidation(t *testing.T) {
	muts := map[string]func(b *Binding){
		"schema":     func(b *Binding) { b.Schema = "x" },
		"repository": func(b *Binding) { b.Repository = "norepo" },
		"repo empty": func(b *Binding) { b.Repository = "" },
		"run":        func(b *Binding) { b.Run = "12a" },
		"run empty":  func(b *Binding) { b.Run = "" },
		"attempt":    func(b *Binding) { b.Attempt = 0 },
		"revision":   func(b *Binding) { b.Revision = "zz" },
		"count":      func(b *Binding) { b.ShardCount = 0 },
		"count big":  func(b *Binding) { b.ShardCount = MaxShards + 1 },
		"index":      func(b *Binding) { b.ShardIndex = 12 },
		"neg index":  func(b *Binding) { b.ShardIndex = -1 },
		"recorder":   func(b *Binding) { b.Recorder = 0 },
		"job start":  func(b *Binding) { b.JobStart = -1 },
		"digest":     func(b *Binding) { b.GraphSHA256 = "x" },
		"text":       func(b *Binding) { b.Kernel = "a\nb" },
	}
	for name, mut := range muts {
		b := goodBinding()
		mut(&b)
		if b.Validate() == nil {
			t.Errorf("%s accepted", name)
		}
		if _, err := EncodeBinding(b); err == nil {
			t.Errorf("%s encoded", name)
		}
	}
	b := goodBinding()
	b.Revision = strings.Repeat("b", 64) // a SHA-256 repository
	if err := b.Validate(); err != nil {
		t.Error(err)
	}
}

func TestHostFactsValidation(t *testing.T) {
	muts := map[string]func(f *HostFacts){
		"schema":      func(f *HostFacts) { f.Schema = "x" },
		"no image":    func(f *HostFacts) { f.Image = "" },
		"image text":  func(f *HostFacts) { f.Image = "a\nb" },
		"no root":     func(f *HostFacts) { f.Root = "" },
		"rel root":    func(f *HostFacts) { f.Root = "work/r" },
		"rel home":    func(f *HostFacts) { f.Home = "h" },
		"rel tmpdir":  func(f *HostFacts) { f.Tmpdir = "t" },
		"arg text":    func(f *HostFacts) { f.ExecutorArgs = []string{"a\nb"} },
		"env no =":    func(f *HostFacts) { f.Env = []string{"LANG"} },
		"env no name": func(f *HostFacts) { f.Env = []string{"=x"} },
		"env path":    func(f *HostFacts) { f.Env = []string{"Path=/bin"} },
		"env repeat":  func(f *HostFacts) { f.Env = []string{"A=1", "A=2"} },
		"env utf8":    func(f *HostFacts) { f.Env = []string{"A=\xff"} },
		"too many":    func(f *HostFacts) { f.Env = make([]string, MaxEnv+1) },
	}
	for name, mut := range muts {
		f := goodFacts()
		mut(&f)
		if f.Validate() == nil {
			t.Errorf("%s accepted", name)
		}
		if _, err := EncodeHostFacts(f); err == nil {
			t.Errorf("%s encoded", name)
		}
	}
	// Home and TMPDIR may be empty (unset on the runner).
	f := goodFacts()
	f.Home, f.Tmpdir, f.ExecutorArgs, f.Env = "", "", nil, nil
	if _, err := EncodeHostFacts(f); err != nil {
		t.Error(err)
	}
}

func TestHeaderValidation(t *testing.T) {
	ok := ShardHeader{Schema: SchemaShard, BindingID: d1, ShardIndex: 0, ShardCount: 2, Void: []string{"a", "b"}}
	muts := map[string]func(h *ShardHeader){
		"schema":    func(h *ShardHeader) { h.Schema = "x" },
		"binding":   func(h *ShardHeader) { h.BindingID = "x" },
		"count":     func(h *ShardHeader) { h.ShardCount = 0 },
		"index":     func(h *ShardHeader) { h.ShardIndex = 2 },
		"neg":       func(h *ShardHeader) { h.PostInventoryUnexplained = -1 },
		"void sort": func(h *ShardHeader) { h.Void = []string{"b", "a"} },
		"void text": func(h *ShardHeader) { h.Void = []string{"a\nb"} },
	}
	for name, mut := range muts {
		h := ok
		mut(&h)
		if h.Validate() == nil {
			t.Errorf("%s accepted", name)
		}
		if _, err := EncodeHeader(h); err == nil {
			t.Errorf("%s encoded", name)
		}
	}
	for _, tok := range []string{"tracked-changed", "state-preloaded", "survivor", "records-too-large"} {
		if !KnownVoid(tok) {
			t.Errorf("%s unknown", tok)
		}
	}
	if KnownVoid("nope") {
		t.Error("nope known")
	}
	if ReasonKind("daemon:127.0.0.1:80") != "daemon" || ReasonKind("failed") != "failed" {
		t.Error("ReasonKind")
	}
}

func TestInventoryValidation(t *testing.T) {
	if (&Inventory{Schema: "x"}).Validate() == nil {
		t.Error("schema")
	}
	if (&Inventory{Schema: SchemaInventory, Paths: []string{"b", "a"}}).Validate() == nil {
		t.Error("unsorted")
	}
	if _, err := EncodeInventory(Inventory{Schema: "x"}); err == nil {
		t.Error("encoded")
	}
}

func TestReadLinesLimits(t *testing.T) {
	collect := func(in string, lim Limits) ([]string, error) {
		var got []string
		err := ReadLines(strings.NewReader(in), lim, func(l []byte) error { got = append(got, string(l)); return nil })
		return got, err
	}
	if got, err := collect("a\nbb\nccc", Limits{MaxLine: 3, MaxFile: 100}); err != nil || len(got) != 3 || got[2] != "ccc" {
		t.Fatalf("%v %v", got, err)
	}
	if _, err := collect("a\nbbbb\n", Limits{MaxLine: 3, MaxFile: 100}); err == nil {
		t.Error("over-long line accepted")
	}
	if _, err := collect("a\nbbbb", Limits{MaxLine: 3, MaxFile: 100}); err == nil {
		t.Error("over-long unterminated line accepted")
	}
	if _, err := collect("aaa\nbbb\nccc\n", Limits{MaxLine: 8, MaxFile: 8}); err == nil {
		t.Error("over-long file accepted")
	}
	if _, err := collect("a\n\nb\n", Limits{MaxLine: 8, MaxFile: 100}); err == nil {
		t.Error("empty line accepted")
	}
	if _, err := collect("a\n", Limits{MaxLine: 8, MaxFile: 100}); err != nil {
		t.Error(err)
	}
	// A line longer than the reader's buffer but within the cap is fine, and
	// the cap holds for an unterminated stream larger than the buffer.
	long := strings.Repeat("x", 200000)
	if got, err := collect(long+"\n", Limits{MaxLine: 300000, MaxFile: 1 << 20}); err != nil || len(got[0]) != len(long) {
		t.Fatalf("%v", err)
	}
	if _, err := collect(long, Limits{MaxLine: 100000, MaxFile: 1 << 20}); err == nil {
		t.Error("cap not enforced across buffer refills")
	}
	// The callback's error is reported with the line number.
	err := ReadLines(strings.NewReader("a\nb\n"), DefaultLimits, func(l []byte) error {
		if string(l) == "b" {
			return io.ErrUnexpectedEOF
		}
		return nil
	})
	if err == nil || !strings.Contains(err.Error(), "line 2") {
		t.Errorf("%v", err)
	}
	if err := ReadLines(iotest.ErrReader(errors.New("boom")), DefaultLimits, func([]byte) error { return nil }); err == nil {
		t.Error("read error swallowed")
	}
}

func TestStepsDigest(t *testing.T) {
	seen := map[string]string{}
	for _, l := range [][]string{nil, {}, {""}, {"a"}, {"a", ""}, {"ab", "c"}, {"a", "bc"}, {"abc"}, {"c", "ab"}} {
		d := StepsDigest(l)
		if len(d) != 64 {
			t.Fatal(d)
		}
		key := strings.Join(l, "|") + "#" + string(rune('0'+len(l)))
		if prev, dup := seen[d]; dup && prev != key {
			// nil and {} are the same list by design.
			if !(len(l) == 0 && strings.HasSuffix(prev, "#0")) {
				t.Errorf("%q collides with %q", key, prev)
			}
		}
		seen[d] = key
	}
	if StepsDigest(nil) != StepsDigest([]string{}) {
		t.Error("nil and empty differ")
	}
	if StepsDigest([]string{"a"}) != StepsDigest([]string{"a"}) {
		t.Error("not deterministic")
	}
}

func TestPartitionDigest(t *testing.T) {
	a := []ShardPlan{{0, []string{"b", "a"}, []string{"a"}}, {1, []string{"c"}, []string{"c"}}}
	b := []ShardPlan{{1, []string{"c"}, []string{"c"}}, {0, []string{"a", "b"}, []string{"a"}}}
	da, err := PartitionDigest(a)
	if err != nil {
		t.Fatal(err)
	}
	db, _ := PartitionDigest(b)
	if da != db {
		t.Error("the digest depends on order")
	}
	c := []ShardPlan{{0, []string{"a", "b"}, []string{"b"}}, {1, []string{"c"}, []string{"c"}}}
	if dc, _ := PartitionDigest(c); dc == da {
		t.Error("a different assignment gives the same digest")
	}
	if _, err := PartitionDigest([]ShardPlan{{0, []string{"a", "a"}, nil}}); err == nil {
		t.Error("duplicate package accepted")
	}
	if _, err := PartitionDigest([]ShardPlan{{0, []string{"a"}, []string{"a", "a"}}}); err == nil {
		t.Error("duplicate assigned accepted")
	}
	if _, err := PartitionDigest([]ShardPlan{{0, nil, nil}, {0, nil, nil}}); err == nil {
		t.Error("duplicate shard accepted")
	}
	if d, err := PartitionDigest(nil); err != nil || len(d) != 64 {
		t.Error(err)
	}
	if short(strings.Repeat("é", 100)) == "" || short("a\nb") != "a?b" {
		t.Error("short")
	}
}

func TestCompleteOfCopiesTheClassifiersAnswer(t *testing.T) {
	if got := CompleteOf(RawRecord{WritesComplete: true}); got == nil || !*got {
		t.Error("a complete record must give true")
	}
	if got := CompleteOf(RawRecord{}); got == nil || *got {
		t.Error("the zero record fails closed: false, never absent")
	}
}

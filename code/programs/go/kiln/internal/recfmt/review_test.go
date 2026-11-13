package recfmt

// Regression tests for the D13 review's decoder findings (PoC zz_fmt): every
// input below was ACCEPTED before the fixes.

import (
	"bytes"
	"strings"
	"testing"
)

func TestReviewDecodeRefusals(t *testing.T) {
	line := string(mustLine(t, goodRecord("a")))
	repl := func(from, to string) string {
		if !strings.Contains(line, from) {
			t.Fatalf("fixture drifted: %q not in %s", from, line)
		}
		return strings.Replace(line, from, to, 1)
	}
	cases := map[string]string{
		// L-7: bytes that are not text, or that are not what they say.
		"invalid utf8 in a path": repl(`"writes":[]`, "\"writes\":[\"a\xff\"]"),
		"invalid utf8 anywhere":  repl(`"language":"go"`, "\"language\":\"g\xffo\""),
		"NUL in a path":          repl(`"writes":[]`, `"writes":["a\u0000b"]`),
		"U+FFFD in a path":       repl(`"writes":[]`, `"writes":["a�b"]`),
		"literal U+FFFD":         repl(`"writes":[]`, "\"writes\":[\"a\xef\xbf\xbdb\"]"),
		"path over the cap":      repl(`"writes":[]`, `"writes":["`+strings.Repeat("x", MaxPath+1)+`"]`),
		"path too deep":          repl(`"writes":[]`, `"writes":["`+strings.Repeat("q/", MaxDepth)+`q"]`),
		// One spelling: no BOM, padding or CR around the value.
		"BOM":           "\xef\xbb\xbf" + line,
		"leading space": " " + line,
		"trailing tab":  line + "\t",
		"CR":            line + "\r",
		// Keys and values.
		"writes_complete null": repl(`"record":`, `"writes_complete":null,"record":`),
		"entry bad prefix":     repl(`"entries":["pkg:a"]`, `"entries":["pkg:a","zzz:q"]`),
		"entry trailing slash": repl(`"entries":["pkg:a"]`, `"entries":["pkg:a","pkg:a/"]`),
		"entry weak:":          repl(`"entries":["pkg:a"]`, `"entries":["pkg:a","weak:00"]`),
		"steps shorter":        repl(`"steps":[{"wall_ms":5,"trace_bytes":100,"clean":true}]`, `"steps":[]`),
		"steps longer":         repl(`"steps_run":1`, `"steps_run":0`),
		"status case":          repl(`"status":"built"`, `"status":"Built"`),
		"status odd":           repl(`"status":"built"`, `"status":"built!"`),
	}
	for name, in := range cases {
		if _, err := DecodeRecord([]byte(in)); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	// A depth just under the cap is fine.
	if _, err := DecodeRecord([]byte(repl(`"writes":[]`, `"writes":["`+strings.Repeat("q/", MaxDepth-2)+`q"]`))); err != nil {
		t.Error(err)
	}
	// Whole files: a stray CR or a blank tail line.
	h, _ := EncodeHeader(ShardHeader{Schema: SchemaShard, BindingID: d1, ShardCount: 1})
	bind, _ := EncodeBinding(goodBinding())
	facts, _ := EncodeHostFacts(goodFacts())
	inv, _ := EncodeInventory(Inventory{Schema: SchemaInventory})
	for name, body := range map[string]string{
		"crlf lines": string(h) + "\r\n",
		"two header": string(h) + "\n" + string(h) + "\n",
	} {
		if _, err := DecodeShard(0, ShardFiles{bind, facts, inv, bytes.NewReader([]byte(body))}, DefaultLimits); err == nil {
			t.Errorf("%s accepted", name)
		}
	}
}

// M-2: the header's new count is a required key and survives a round trip.
func TestReviewHeaderTrackedChangedUnexplained(t *testing.T) {
	h := ShardHeader{Schema: SchemaShard, BindingID: d1, ShardIndex: 0, ShardCount: 2, TrackedChangedUnexplained: 3}
	b, err := EncodeHeader(h)
	if err != nil {
		t.Fatal(err)
	}
	got, err := DecodeHeader(b)
	if err != nil || got.TrackedChangedUnexplained != 3 {
		t.Fatalf("%v %+v", err, got)
	}
	without := strings.Replace(string(b), `,"tracked_changed_unexplained":3`, ``, 1)
	if without == string(b) {
		t.Fatal("fixture drifted")
	}
	if _, err := DecodeHeader([]byte(without)); err == nil {
		t.Error("a header without tracked_changed_unexplained was accepted")
	}
	h.TrackedChangedUnexplained = -1
	if _, err := EncodeHeader(h); err == nil {
		t.Error("a negative count was accepted")
	}
}

// L-4: validation messages do not depend on map iteration order.
func TestReviewValidationMessagesAreStable(t *testing.T) {
	seen := map[string]bool{}
	for i := 0; i < 200; i++ {
		b := goodBinding()
		b.KilnSHA256, b.BuildToolSHA256, b.SandboxConfigSHA256, b.GraphSHA256 = "x", "x", "x", "x"
		seen[b.Validate().Error()] = true
		b = goodBinding()
		b.Kernel, b.StraceVersion = "a\nb", "c\nd"
		seen["t"+b.Validate().Error()] = true
	}
	if len(seen) != 2 {
		t.Errorf("%d distinct messages: %v", len(seen), seen)
	}
}

package key

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"os"
	"strings"
	"testing"
)

func h32(b ...byte) [32]byte { return sha256.Sum256(b) }

// The four state digests are pinned by hand to explicit byte strings, so the
// vectors are not the only thing standing between a refactor and a changed tag.
func TestStateDigestsByHand(t *testing.T) {
	content := sha256.Sum256([]byte("c"))
	link, err := LinkDigest("t")
	if err != nil {
		t.Fatal(err)
	}
	for _, c := range []struct {
		name string
		got  [32]byte
		pre  []byte
	}{
		{"absent", AbsentDigest(), []byte{0}},
		{"directory", DirectoryDigest(), []byte{1}},
		{"file", FileDigest(false, content), append([]byte{2, 0}, content[:]...)},
		{"exec file", FileDigest(true, content), append([]byte{2, 1}, content[:]...)},
		{"link", link, []byte{3, 0, 0, 0, 0, 0, 0, 0, 1, 't'}},
	} {
		if want := sha256.Sum256(c.pre); c.got != want {
			t.Errorf("%s: got %x want %x", c.name, c.got, want)
		}
	}
	seen := map[[32]byte]string{}
	for n, d := range map[string][32]byte{
		"absent": AbsentDigest(), "dir": DirectoryDigest(), "file": FileDigest(false, content),
		"exec": FileDigest(true, content), "link": link, "missing": MissingPackageDigest(),
	} {
		if o, dup := seen[d]; dup {
			t.Errorf("%s and %s share a digest", n, o)
		}
		seen[d] = n
	}
}

func TestLinkDigestRefusesInvalidUTF8(t *testing.T) {
	if _, err := LinkDigest("a\xffb"); err == nil || !strings.Contains(err.Error(), "UTF-8") {
		t.Fatalf("got %v", err)
	}
}

func TestMissingPackageDigestByHand(t *testing.T) {
	want := sha256.Sum256(append([]byte{0, 0, 0, 0, 0, 0, 0, 20}, "kiln-missing-package"...))
	if MissingPackageDigest() != want {
		t.Fatal("missing-package digest is not SHA-256(str(\"kiln-missing-package\"))")
	}
}

func TestListingDigest(t *testing.T) {
	d, err := ListingDigest([]ListingEntry{{"B", ListFile}, {"a", ListDir}})
	if err != nil {
		t.Fatal(err)
	}
	want := sha256.Sum256([]byte("\x10\x00\x00\x00\x00\x00\x00\x00\x02" +
		"\x00\x00\x00\x00\x00\x00\x00\x01B\x01" + "\x00\x00\x00\x00\x00\x00\x00\x01a\x03"))
	if d != want {
		t.Fatalf("got %x want %x", d, want)
	}
	notDir := NotDirectoryListingDigest()
	if notDir != sha256.Sum256([]byte{0x11}) {
		t.Fatal("not-a-directory digest is not SHA-256(11)")
	}
	// Tagged: no listing equals the not-a-directory digest or any state digest.
	for _, o := range [][32]byte{notDir, AbsentDigest(), DirectoryDigest(), MissingPackageDigest()} {
		if d == o {
			t.Fatal("listing digest collides with a tagged digest")
		}
	}
	if notDir == AbsentDigest() {
		t.Fatal("not-a-directory listing equals the absent state")
	}
	if _, err := ListingDigest(nil); err == nil {
		t.Fatal("a tracked directory with no children was accepted")
	}
	// A kind change and a name change both move the digest.
	k, _ := ListingDigest([]ListingEntry{{"B", ListFile}, {"a", ListFile}})
	n, _ := ListingDigest([]ListingEntry{{"B", ListFile}, {"b", ListDir}})
	if k == d || n == d || k == n {
		t.Fatal("listing digest ignores a kind or a name")
	}
}

func TestActionCloneIsDeep(t *testing.T) {
	host := "h"
	a := Action{HostImageID: &host, Recipe: []byte{1}, Inputs: []FileEntry{{Path: "a", Digest: []byte{1}}},
		Reads: []FileEntry{{Path: "b", Digest: []byte{2}}}, Consumed: []Consumed{{Target: "t", Key: []byte{3}}},
		Tools: []Tool{{Name: "n", Identity: []byte{4}}}, Env: []EnvVar{{Name: "e", Value: "v"}}}
	c := a.Clone()
	*c.HostImageID = "x"
	c.Recipe[0], c.Inputs[0].Digest[0], c.Reads[0].Digest[0] = 9, 9, 9
	c.Consumed[0].Key[0], c.Tools[0].Identity[0] = 9, 9
	c.Env[0].Value = "x"
	if host != "h" || a.Recipe[0] != 1 || a.Inputs[0].Digest[0] != 1 || a.Reads[0].Digest[0] != 2 ||
		a.Consumed[0].Key[0] != 3 || a.Tools[0].Identity[0] != 4 || a.Env[0].Value != "v" {
		t.Fatal("Clone shares memory with the original")
	}
	if z := (Action{}).Clone(); z.HostImageID != nil || z.Inputs != nil || z.Consumed != nil || z.Tools != nil {
		t.Fatal("Clone of an empty action invents sections")
	}
}

func TestListingDigestRefusals(t *testing.T) {
	for _, c := range []struct {
		name string
		es   []ListingEntry
		want string
	}{
		{"unsorted", []ListingEntry{{"b", 1}, {"a", 1}}, "not sorted"},
		{"case order", []ListingEntry{{"a", 1}, {"B", 1}}, "not sorted"},
		{"duplicate", []ListingEntry{{"a", 1}, {"a", 3}}, "duplicate"},
		{"empty name", []ListingEntry{{"", 1}}, "not a child name"},
		{"slash", []ListingEntry{{"a/b", 1}}, "not a child name"},
		{"bad utf8", []ListingEntry{{"a\xff", 1}}, "UTF-8"},
		{"kind 0", []ListingEntry{{"a", 0}}, "unknown kind"},
		{"kind 4", []ListingEntry{{"a", 4}}, "unknown kind"},
	} {
		if _, err := ListingDigest(c.es); err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("%s: got %v, want %q", c.name, err, c.want)
		}
	}
}

func TestTracedPackageDigest(t *testing.T) {
	own := h32(1)
	none, err := TracedPackageDigest(own, nil)
	if err != nil {
		t.Fatal(err)
	}
	pre := append([]byte("\x00\x00\x00\x00\x00\x00\x00\x0fkiln-traced-pkg\x00\x00\x00\x00\x00\x00\x00\x20"), own[:]...)
	pre = append(pre, make([]byte, 8)...)
	if none != sha256.Sum256(pre) {
		t.Fatal("T(q) with no nested packages does not match the spec's byte layout")
	}
	one, _ := TracedPackageDigest(own, []string{"a/b"})
	two, _ := TracedPackageDigest(own, []string{"a/b", "a/c"})
	other, _ := TracedPackageDigest(h32(2), nil)
	if one == none || two == one || other == none {
		t.Fatal("T(q) ignores the nested list or D(q)")
	}
	again, _ := TracedPackageDigest(own, []string{"a/b", "a/c"})
	if again != two {
		t.Fatal("T(q) is not deterministic")
	}
	for name, nested := range map[string][]string{
		"unsorted": {"b", "a"}, "duplicate": {"a", "a"}, "dotdot": {"../x"}, "empty": {""}, "slash": {"/x"},
	} {
		if _, err := TracedPackageDigest(own, nested); err == nil {
			t.Errorf("%s nested list accepted", name)
		}
	}
}

func TestCheckEntryName(t *testing.T) {
	for _, ok := range []string{"pkg:a/b", "path:x", "path:a/b.go", "list:.", "list:a/b", "pkg:code/packages/go/kiln"} {
		if err := CheckEntryName(ok); err != nil {
			t.Errorf("%q refused: %v", ok, err)
		}
	}
	for _, bad := range []string{"", "weak:", "weak:abcd", "tool:x", "policy:env.json", "glob:a", "dir:a", "a/b", "PKG:a",
		"pkg:", "path:", "list:", "path:.", "pkg:../a", "path:/abs", "path:a//b", "path:a/", "list:a/..", "path:a\xff"} {
		if err := CheckEntryName(bad); err == nil {
			t.Errorf("%q accepted", bad)
		}
	}
	long := "weak:" + strings.Repeat("x", 500)
	if err := CheckEntryName(long); err == nil || len(err.Error()) > 200 {
		t.Errorf("a long name is not clipped: %v", err)
	}
}

func TestStrongConsumed(t *testing.T) {
	w := h32(9)
	es := []EntryDigest{{"pkg:q", h32(1)}, {"list:.", h32(2)}, {"path:a", h32(3)}}
	got, err := StrongConsumed(w, es)
	if err != nil {
		t.Fatal(err)
	}
	var targets []string
	for _, c := range got {
		targets = append(targets, c.Target)
		if c.Phase != PhaseBuild || len(c.Key) != 32 {
			t.Errorf("%s: phase %q key %d bytes", c.Target, c.Phase, len(c.Key))
		}
	}
	if strings.Join(targets, ",") != "list:.,path:a,pkg:q,weak:" {
		t.Fatalf("targets %v", targets)
	}
	if hex.EncodeToString(got[3].Key) != hex.EncodeToString(w[:]) {
		t.Fatal("weak: entry does not hold the weak key")
	}
	// The result is encodable as it stands (sorted, unique, 32-byte keys).
	if _, err := ConsumedDigest(got); err != nil {
		t.Fatal(err)
	}
	// The input slice is not reordered.
	if es[0].Name != "pkg:q" {
		t.Fatal("StrongConsumed reordered its argument")
	}
	if _, err := StrongConsumed(w, []EntryDigest{{"weak:" + hex.EncodeToString(w[:]), w}}); err == nil {
		t.Fatal("a weak: manifest entry was accepted")
	}
	if _, err := StrongConsumed(w, []EntryDigest{{"path:a", w}, {"path:a", w}}); err == nil || !strings.Contains(err.Error(), "twice") {
		t.Fatalf("duplicate: %v", err)
	}
}

// A weak key and a strong key can never collide: the strong key has a weak:
// consumed entry and the weak key never does, and each key's preimage commits
// to its consumed section.
func TestWeakAndStrongNeverCollide(t *testing.T) {
	weak := weakAction()
	wa, _ := weak.ToAction()
	wk, err := Key(wa)
	if err != nil {
		t.Fatal(err)
	}
	for _, c := range wa.Consumed {
		if c.Target == WeakTarget {
			t.Fatal("a weak key carries a weak: entry")
		}
	}
	// Even a strong key over an EMPTY manifest differs from its weak key.
	cons, _ := StrongConsumed(wk, nil)
	sa := wa
	sa.Consumed = cons
	sk, err := Key(sa)
	if err != nil {
		t.Fatal(err)
	}
	if sk == wk {
		t.Fatal("strong key over an empty manifest equals the weak key")
	}
}

func TestStrongKeyMovesOnlyWithItsEntries(t *testing.T) {
	wa, _ := weakAction().ToAction()
	wk, _ := Key(wa)
	keyOf := func(es []EntryDigest) [32]byte {
		cons, err := StrongConsumed(wk, es)
		if err != nil {
			t.Fatal(err)
		}
		a := wa
		a.Consumed = cons
		k, err := Key(a)
		if err != nil {
			t.Fatal(err)
		}
		return k
	}
	base := []EntryDigest{{"pkg:q", h32(1)}, {"path:a", h32(2)}, {"list:d", h32(3)}}
	k0 := keyOf(base)
	if keyOf([]EntryDigest{base[2], base[0], base[1]}) != k0 {
		t.Error("entry order changed the key")
	}
	for i := range base {
		moved := append([]EntryDigest(nil), base...)
		moved[i].Digest = h32(99)
		if keyOf(moved) == k0 {
			t.Errorf("changing entry %s did not change the key", base[i].Name)
		}
	}
	if keyOf(base[:2]) == k0 {
		t.Error("dropping an entry did not change the key")
	}
	// Same digest under a different entry kind is a different key.
	swapped := []EntryDigest{{"pkg:a", h32(1)}, {"path:q", h32(2)}, {"list:d", h32(3)}}
	if keyOf(swapped) == k0 {
		t.Error("entry names do not matter")
	}
}

func TestDigestVectorErrors(t *testing.T) {
	good := sum("x")
	for name, v := range map[string]DigestVector{
		"unknown kind":     {Kind: "state-weird"},
		"bad exec":         {Kind: DigestFile, Exec: 2, ContentHex: good},
		"short content":    {Kind: DigestFile, ContentHex: "00"},
		"non-hex content":  {Kind: DigestFile, ContentHex: "zz"},
		"bad link":         {Kind: DigestLink, Target: "a\xff"},
		"stray entries 2":  {Kind: DigestNotDir, Entries: []ListingJSON{{"a", 1}}},
		"empty listing":    {Kind: DigestListing},
		"bad own":          {Kind: DigestTracedPkg, OwnDigestHex: "0"},
		"unsorted nested":  {Kind: DigestTracedPkg, OwnDigestHex: good, Nested: []string{"b", "a"}},
		"unsorted listing": {Kind: DigestListing, Entries: []ListingJSON{{"b", 1}, {"a", 1}}},
		"stray content":    {Kind: DigestAbsent, ContentHex: good},
		"stray target":     {Kind: DigestAbsent, Target: "x"},
		"stray entries":    {Kind: DigestAbsent, Entries: []ListingJSON{{"a", 1}}},
		"stray own":        {Kind: DigestAbsent, OwnDigestHex: good},
		"stray nested":     {Kind: DigestDirectory, Nested: []string{"a"}},
	} {
		if _, err := v.Compute(); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
}

func parsedVectors(t *testing.T) VectorFile {
	t.Helper()
	data, err := os.ReadFile(vectorPath)
	if err != nil {
		t.Fatal(err)
	}
	vf, err := ParseVectors(data)
	if err != nil {
		t.Fatal(err)
	}
	return vf
}

// The committed file verifies, and every traced vector kind is exercised by it.
func TestCommittedTracedVectors(t *testing.T) {
	vf := parsedVectors(t)
	if fails := vf.Verify(); len(fails) > 0 {
		t.Fatal(strings.Join(fails, "\n"))
	}
	kinds := map[string]bool{}
	for _, d := range vf.Digests {
		kinds[d.Kind] = true
	}
	for _, k := range []string{DigestAbsent, DigestDirectory, DigestFile, DigestLink, DigestMissingPkg, DigestListing, DigestNotDir, DigestTracedPkg} {
		if !kinds[k] {
			t.Errorf("no %s vector", k)
		}
	}
	// T(q) with and without nested packages must differ.
	var with, without string
	for _, d := range vf.Digests {
		switch d.Name {
		case "traced-package-nested":
			with = d.DigestHex
		case "traced-package-no-nested":
			without = d.DigestHex
		}
	}
	if with == "" || with == without {
		t.Error("T(q) vectors do not show the nested list mattering")
	}
}

// Every way a traced vector can be wrong is reported, never skipped.
func TestVerifyReportsTracedMismatches(t *testing.T) {
	mutate := map[string]func(vf *VectorFile){
		"digest":          func(vf *VectorFile) { vf.Digests[0].DigestHex = sum("wrong") },
		"digest error":    func(vf *VectorFile) { vf.Digests[0].Kind = "bogus" },
		"consumed target": func(vf *VectorFile) { vf.StrongConsumed[0].Consumed[0].Target = "pkg:other" },
		"consumed key":    func(vf *VectorFile) { vf.StrongConsumed[0].Consumed[0].KeyHex = sum("wrong") },
		"consumed count":  func(vf *VectorFile) { vf.StrongConsumed[0].Consumed = vf.StrongConsumed[0].Consumed[1:] },
		"consumed weak":   func(vf *VectorFile) { vf.StrongConsumed[0].WeakKeyHex = "00" },
		"consumed entry":  func(vf *VectorFile) { vf.StrongConsumed[0].Entries[0].DigestHex = "zz" },
		"consumed refuse": func(vf *VectorFile) { vf.StrongConsumed[0].Entries[0].Name = "weak:x" },
		"refused accepted": func(vf *VectorFile) {
			vf.RefusedEntries[0].Entries[0].Name = "pkg:fine"
		},
		"refused wrong error": func(vf *VectorFile) { vf.RefusedEntries[0].Error = "something else" },
		"refused empty error": func(vf *VectorFile) { vf.RefusedEntries[0].Error = "" },
		"refused bad weak":    func(vf *VectorFile) { vf.RefusedEntries[0].WeakKeyHex = "00" },
		"refused bad entry":   func(vf *VectorFile) { vf.RefusedEntries[0].Entries[0].DigestHex = "zz" },
		"no digests":          func(vf *VectorFile) { vf.Digests = nil },
		"no strong":           func(vf *VectorFile) { vf.StrongConsumed = nil },
		"no refused":          func(vf *VectorFile) { vf.RefusedEntries = nil },
	}
	for name, f := range mutate {
		vf := parsedVectors(t)
		// Copy the slices a mutation edits in place, so cases stay independent.
		raw, _ := json.Marshal(vf)
		_ = json.Unmarshal(raw, &vf)
		f(&vf)
		if fails := vf.Verify(); len(fails) == 0 {
			t.Errorf("%s: mutation went unnoticed", name)
		}
	}
}

func TestParseVectorsRejectsUnknownTracedField(t *testing.T) {
	data, err := os.ReadFile(vectorPath)
	if err != nil {
		t.Fatal(err)
	}
	bad := strings.Replace(string(data), `"digest_hex": "`, `"digest_hexx": "`, 1)
	if _, err := ParseVectors([]byte(bad)); err == nil {
		t.Fatal("a misspelt field in a digest vector was accepted")
	}
}

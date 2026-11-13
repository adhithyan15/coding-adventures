package key

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"io"
	"os"
	"runtime"
	"strings"
	"testing"
)

func mustHex(t *testing.T, s string) []byte {
	t.Helper()
	b, err := hex.DecodeString(strings.ReplaceAll(s, " ", ""))
	if err != nil {
		t.Fatal(err)
	}
	return b
}

func TestBuildingBlocks(t *testing.T) {
	if got := putU8(nil, 1); !bytes.Equal(got, []byte{1}) {
		t.Errorf("u8: %x", got)
	}
	if got := hex.EncodeToString(putU64(nil, 3)); got != "0000000000000003" {
		t.Errorf("u64: %s", got)
	}
	if got := hex.EncodeToString(putBytes(nil, []byte("hi"))); got != "00000000000000026869" {
		t.Errorf("bytes: %s", got)
	}
	if got := hex.EncodeToString(putStr(nil, "é")); got != "0000000000000002c3a9" {
		t.Errorf("str: %s", got)
	}
	if got := hex.EncodeToString(putU64(nil, 0)); got != "0000000000000000" { // list([])
		t.Errorf("empty list: %s", got)
	}
	if got := hex.EncodeToString(putU64(nil, 1<<40)); got != "0000010000000000" {
		t.Errorf("u64 big: %s", got)
	}
}

// TestMinimalActionByHand writes the preimage of the minimal action as
// explicit byte literals, independent of the encoder, so the golden vectors
// are not purely self-referential.
func TestMinimalActionByHand(t *testing.T) {
	want := mustHex(t, ""+
		"0000000000000008 6b696c6e2d6b6579"+ // str("kiln-key")
		"0000000000000001"+ // schema 1
		"0000000000000000"+ // semantics 0
		"0000000000000008 706c6174666f726d"+ // "platform"
		"0000000000000005 6c696e7578"+ // "linux"
		"0000000000000003 783634"+ // "x64"
		"0000000000000004 686f7374 00"+ // "host", tag 0
		"0000000000000006 616374696f6e"+ // "action"
		"0000000000000001 74"+ // target "t"
		"0000000000000005 6275696c64"+ // phase "build"
		"0000000000000006 726563697065"+ // "recipe"
		"0000000000000001 72"+ // bytes("r")
		"0000000000000006 696e70757473 0000000000000000"+ // "inputs", empty
		"0000000000000005 7265616473 0000000000000000"+ // "reads", empty
		"0000000000000008 636f6e73756d6564 0000000000000000"+ // "consumed", empty
		"0000000000000005 746f6f6c73 0000000000000000"+ // "tools", empty
		"0000000000000003 656e76 0000000000000000") // "env", empty
	a, err := ActionJSON{OS: "linux", Arch: "x64", Target: "t", Phase: "build", RecipeHex: "72"}.ToAction()
	if err != nil {
		t.Fatal(err)
	}
	got, err := Preimage(a)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(got, want) {
		t.Fatalf("preimage\n got  %x\n want %x", got, want)
	}
	k, _ := Key(a)
	if len(Hex(k)) != 64 || Hex(k) != strings.ToLower(Hex(k)) {
		t.Errorf("Hex: %s", Hex(k))
	}
}

func TestLegacyRecipe(t *testing.T) {
	want := mustHex(t, "000000000000000c 6c65676163792d7368656c6c"+ // "legacy-shell"
		"0000000000000001"+
		"0000000000000005 4255494c44"+ // "BUILD"
		"0000000000000002 6162")
	if got := LegacyRecipe([]byte("ab"), nil); !bytes.Equal(got, want) {
		t.Errorf("without windows: %x", got)
	}
	with := LegacyRecipe([]byte("ab"), []byte{})
	if !bytes.Contains(with, []byte("BUILD_windows")) || with[8+12+7] != 2 {
		t.Errorf("with empty windows file must be included with count 2: %x", with)
	}
	if bytes.Equal(with, LegacyRecipe([]byte("ab"), nil)) {
		t.Error("absent and empty BUILD_windows must differ")
	}
}

func mustKey(t *testing.T, j ActionJSON) string {
	t.Helper()
	a, err := j.ToAction()
	if err != nil {
		t.Fatal(err)
	}
	k, err := Key(a)
	if err != nil {
		t.Fatal(err)
	}
	return Hex(k)
}

func TestCollisionPairsDiffer(t *testing.T) {
	d := validDefs()
	byName := map[string]ActionJSON{}
	for _, v := range d {
		byName[v.name] = v.action
	}
	for _, pair := range [][2]string{
		{"collision-env-a", "collision-env-b"},
		{"collision-target-a", "collision-target-b"},
		{"host-absent", "host-present"},
		{"legacy-build-only", "legacy-build-and-windows"},
	} {
		if mustKey(t, byName[pair[0]]) == mustKey(t, byName[pair[1]]) {
			t.Errorf("%v produce the same key", pair)
		}
	}
	// Tools: name/version boundary.
	x := with(func(a *ActionJSON) { a.Tools = []ToolJSON{{"a,b", "c", 0, ""}} })
	y := with(func(a *ActionJSON) { a.Tools = []ToolJSON{{"a", "b,c", 0, ""}} })
	if mustKey(t, x) == mustKey(t, y) {
		t.Error("tool name/version boundary collides")
	}
}

func TestKeyIsDeterministicAndVersioned(t *testing.T) {
	a, _ := base().ToAction()
	k1, _ := Key(a)
	k2, _ := Key(a)
	if k1 != k2 {
		t.Error("not deterministic")
	}
	a.SemanticsVersion = 1
	k3, _ := Key(a)
	if k3 == k1 {
		t.Error("semantics version must change the key")
	}
}

func TestBackslashIsOrdinaryByte(t *testing.T) {
	if err := checkPath(`a\b`); err != nil {
		t.Errorf("backslash should be an ordinary byte: %v", err)
	}
}

func TestCheckPath(t *testing.T) {
	for _, p := range []string{"a", "a/b", "a.b/c..d", ".hidden", "é/日本"} {
		if err := checkPath(p); err != nil {
			t.Errorf("%q rejected: %v", p, err)
		}
	}
	for _, p := range []string{"", "/a", "a/", "a//b", "./a", "a/.", "..", "a/../b", "a\xffb"} {
		if err := checkPath(p); err == nil {
			t.Errorf("%q accepted", p)
		}
	}
}

func TestValidateErrors(t *testing.T) {
	bad := "\xff"
	cases := map[string]struct {
		mod  func(a *ActionJSON)
		fix  func(a *Action)
		want string
	}{
		"host-utf8":     {fix: func(a *Action) { a.HostImageID = &bad }, want: "host image id"},
		"tool-name":     {fix: func(a *Action) { a.Tools = []Tool{{Name: bad, Kind: 0}} }, want: "tool name"},
		"tool-version":  {fix: func(a *Action) { a.Tools = []Tool{{Name: "x", Version: bad, Kind: 0}} }, want: "tool version"},
		"env-name":      {fix: func(a *Action) { a.Env = []EnvVar{{Name: bad}} }, want: "env name"},
		"env-value":     {fix: func(a *Action) { a.Env = []EnvVar{{Name: "x", Value: bad}} }, want: "env value"},
		"consumed-utf8": {fix: func(a *Action) { a.Consumed = []Consumed{{Target: bad, Phase: "build", Key: make([]byte, 32)}} }, want: "consumed target"},
		"path-utf8":     {fix: func(a *Action) { a.Inputs = []FileEntry{{Path: bad, Kind: 1, Digest: make([]byte, 32)}} }, want: "path"},
		"exec-2":        {fix: func(a *Action) { a.Reads = []FileEntry{{Path: "a", Kind: 1, Exec: 2, Digest: make([]byte, 32)}} }, want: "exec must be"},
		"consumed-phase": {fix: func(a *Action) {
			a.Consumed = []Consumed{{Target: "a", Phase: "x", Key: make([]byte, 32)}}
		}, want: "unknown phase"},
		"reads-bad-kind": {fix: func(a *Action) { a.Reads = []FileEntry{{Path: "a", Kind: 0, Digest: make([]byte, 32)}} }, want: "unknown kind"},
		"unsorted-reads": {fix: func(a *Action) {
			a.Reads = []FileEntry{{Path: "b", Kind: 1, Digest: make([]byte, 32)}, {Path: "a", Kind: 1, Digest: make([]byte, 32)}}
		}, want: "not sorted"},
		"unsorted-env":    {fix: func(a *Action) { a.Env = []EnvVar{{Name: "b"}, {Name: "a"}} }, want: "not sorted"},
		"bad-hex-handled": {fix: func(a *Action) {}, want: ""},
	}
	for name, c := range cases {
		a, _ := base().ToAction()
		c.fix(&a)
		_, err := Preimage(a)
		if c.want == "" {
			if err != nil {
				t.Errorf("%s: %v", name, err)
			}
			continue
		}
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("%s: got %v, want %q", name, err, c.want)
		}
	}
	if _, err := Key(Action{OS: "x"}); err == nil {
		t.Error("Key must propagate validation errors")
	}
}

func TestToActionBadHex(t *testing.T) {
	zz := "zz"
	for name, j := range map[string]ActionJSON{
		"target":   with(func(a *ActionJSON) { a.TargetHex = zz }),
		"recipe":   with(func(a *ActionJSON) { a.RecipeHex = zz }),
		"inputs":   with(func(a *ActionJSON) { a.Inputs = []FileJSON{{"a", 1, 0, zz}} }),
		"reads":    with(func(a *ActionJSON) { a.Reads = []FileJSON{{"a", 1, 0, zz}} }),
		"consumed": with(func(a *ActionJSON) { a.Consumed = []ConsumedJSON{{"a", "build", zz}} }),
		"tools":    with(func(a *ActionJSON) { a.Tools = []ToolJSON{{"a", "1", 2, zz}} }),
	} {
		if _, err := j.ToAction(); err == nil {
			t.Errorf("%s: bad hex accepted", name)
		}
	}
}

func TestGoldenFileRoundTrips(t *testing.T) {
	data, err := os.ReadFile(vectorPath)
	if err != nil {
		t.Fatal(err)
	}
	vf, err := ParseVectors(data)
	if err != nil {
		t.Fatal(err)
	}
	if fails := vf.Verify(); len(fails) != 0 {
		t.Fatalf("%s", strings.Join(fails, "\n"))
	}
	// The committed file must equal what the definitions generate.
	gen := buildVectorFile(t)
	if len(gen.Valid) != len(vf.Valid) || len(gen.Invalid) != len(vf.Invalid) {
		t.Fatalf("file is stale: regenerate with -update")
	}
	for i, v := range gen.Valid {
		if v.KeyHex != vf.Valid[i].KeyHex {
			t.Errorf("%s: stale key in file", v.Name)
		}
	}
	// Spec 6 coverage: names that must be present.
	have := map[string]bool{}
	for _, v := range vf.Valid {
		have[v.Name] = true
	}
	for _, n := range []string{"minimal", "collision-env-a", "unicode-and-byte-order", "link-and-exec",
		"tools-every-kind", "host-absent", "host-present", "legacy-build-only", "legacy-build-and-windows"} {
		if !have[n] {
			t.Errorf("missing vector %s", n)
		}
	}
}

func TestVerifyDetectsProblems(t *testing.T) {
	data, _ := os.ReadFile(vectorPath)
	fresh := func() VectorFile { vf, _ := ParseVectors(data); return vf }

	vf := fresh()
	vf.Valid[0].KeyHex = strings.Repeat("0", 64)
	vf.Valid[1].PreimageHex = "00"
	vf.Invalid[0].Error = "no such text"
	vf.Invalid[1].Action = with(func(a *ActionJSON) {}) // valid action in invalid list
	vf.Valid[2].Action.RecipeHex = "zz"
	vf.Invalid[2].Action.Target, vf.Invalid[2].Action.TargetHex = "", "zz"
	vf.Valid[3].Action.OS = "plan9"
	out := strings.Join(vf.Verify(), "\n")
	for _, w := range []string{"key mismatch", "preimage mismatch", "does not contain", "accepted but must be refused", "recipe_hex", "target_hex", "unexpected error"} {
		if !strings.Contains(out, w) {
			t.Errorf("Verify output missing %q:\n%s", w, out)
		}
	}
	if len((VectorFile{}).Verify()) == 0 {
		t.Error("empty file must fail")
	}
}

func TestParseVectorsErrors(t *testing.T) {
	for name, s := range map[string]string{
		"garbage":       "{",
		"unknown-field": `{"schema_version":1,"bogus":1}`,
		"wrong-schema":  `{"schema_version":2}`,
	} {
		if _, err := ParseVectors([]byte(s)); err == nil {
			t.Errorf("%s accepted", name)
		}
	}
}

func TestParseVectorsTrailingData(t *testing.T) {
	for name, s := range map[string]string{
		"second-document": `{"schema_version":1} {"schema_version":1}`,
		"garbage-after":   `{"schema_version":1} x`,
	} {
		_, err := ParseVectors([]byte(s))
		if err == nil || !strings.Contains(err.Error(), "trailing data after vector document") {
			t.Errorf("%s: %v", name, err)
		}
	}
	// Trailing whitespace is not data.
	if _, err := ParseVectors([]byte("{\"schema_version\":1}\n\n")); err != nil {
		t.Errorf("trailing whitespace refused: %v", err)
	}
}

func TestVerifyRejectsEmptyErrorExpectation(t *testing.T) {
	data, _ := os.ReadFile(vectorPath)
	vf, _ := ParseVectors(data)
	vf.Invalid[0].Error = ""
	out := strings.Join(vf.Verify(), "\n")
	if !strings.Contains(out, `empty "error" field`) {
		t.Errorf("empty error accepted:\n%s", out)
	}
}

func TestTargetAndTargetHexTogetherRefused(t *testing.T) {
	j := with(func(a *ActionJSON) {})
	j.Target = "t"
	j.TargetHex = "74"
	if _, err := j.ToAction(); err == nil || !strings.Contains(err.Error(), "not both") {
		t.Errorf("both target fields accepted: %v", err)
	}
}

func TestRequireRegularStatError(t *testing.T) {
	f, err := os.Open(os.Args[0])
	if err != nil {
		t.Fatal(err)
	}
	f.Close() // Stat on a closed file fails.
	if requireRegular(f) == nil {
		t.Error("closed file accepted")
	}
}

type failingReader struct{}

func (failingReader) Read([]byte) (int, error) { return 0, os.ErrClosed }

func TestReadLimited(t *testing.T) {
	if _, err := readLimited(failingReader{}, 10); err == nil {
		t.Error("read error swallowed")
	}
	// A reader longer than the production cap is refused without a big file.
	big := io.LimitReader(zeroReader{}, MaxVectorFileBytes+10)
	if _, err := readLimited(big, MaxVectorFileBytes); err == nil || !strings.Contains(err.Error(), "larger than") {
		t.Errorf("oversized input accepted: %v", err)
	}
}

type zeroReader struct{}

func (zeroReader) Read(p []byte) (int, error) { clear(p); return len(p), nil }

func TestReadVectorFile(t *testing.T) {
	dir := t.TempDir()
	if _, err := ReadVectorFile(dir); err == nil || !strings.Contains(err.Error(), "not a regular file") {
		t.Errorf("directory accepted: %v", err)
	}
	if _, err := ReadVectorFile(dir + "/missing"); err == nil {
		t.Error("missing file accepted")
	}
	p := dir + "/v.json"
	if err := os.WriteFile(p, []byte("12345"), 0o644); err != nil {
		t.Fatal(err)
	}
	if b, err := readVectorFile(p, 5); err != nil || string(b) != "12345" {
		t.Errorf("exactly-at-limit refused: %q %v", b, err)
	}
	if _, err := readVectorFile(p, 4); err == nil || !strings.Contains(err.Error(), "larger than") {
		t.Errorf("over-limit accepted: %v", err)
	}
	if b, err := ReadVectorFile(p); err != nil || len(b) != 5 {
		t.Errorf("production read: %q %v", b, err)
	}
	if MaxVectorFileBytes != 16<<20 {
		t.Error("production limit must stay 16 MiB")
	}
	// Unreadable file: stat succeeds, open fails. Skipped on Windows, where
	// mode 0o000 only sets the read-only attribute and os.Getuid() returns -1.
	if runtime.GOOS != "windows" && os.Getuid() != 0 {
		q := dir + "/noread.json"
		os.WriteFile(q, []byte("x"), 0o000)
		if _, err := ReadVectorFile(q); err == nil {
			t.Error("unreadable file accepted")
		}
	}
}

func TestValidatePath(t *testing.T) {
	if err := ValidatePath("src/a.go"); err != nil {
		t.Fatal(err)
	}
	if err := ValidatePath("../a"); err == nil {
		t.Fatal("expected error")
	}
}

func TestLegacyRecipeFilesMatchesLegacyRecipe(t *testing.T) {
	b, w := []byte("build\n"), []byte("win\r\n")
	got, err := LegacyRecipeFiles([]NamedFile{{"BUILD", b}, {"BUILD_windows", w}})
	if err != nil || !bytes.Equal(got, LegacyRecipe(b, w)) {
		t.Errorf("two files: %x %v", got, err)
	}
	got, err = LegacyRecipeFiles([]NamedFile{{"BUILD", b}})
	if err != nil || !bytes.Equal(got, LegacyRecipe(b, nil)) {
		t.Errorf("build only: %x %v", got, err)
	}
	other, _ := LegacyRecipeFiles([]NamedFile{{"BUILD_linux", b}})
	if bytes.Equal(other, got) {
		t.Error("file name must be part of the recipe")
	}
}

func TestLegacyRecipeFilesRefusals(t *testing.T) {
	c := []byte("x")
	cases := map[string][]NamedFile{
		"empty list":     nil,
		"empty name":     {{"", c}},
		"invalid utf8":   {{"B\xff", c}},
		"slash":          {{"a/BUILD", c}},
		"backslash":      {{"a\\BUILD", c}},
		"duplicate name": {{"BUILD", c}, {"BUILD", c}},
	}
	for name, files := range cases {
		if _, err := LegacyRecipeFiles(files); err == nil {
			t.Errorf("%s: expected error", name)
		}
	}
}

func TestSectionDigests(t *testing.T) {
	d := bytes.Repeat([]byte{7}, 32)
	es := []FileEntry{{Path: "a", Kind: KindFile, Digest: d}, {Path: "b", Kind: KindLink, Digest: d}}
	want := sha256.Sum256(putFiles(nil, "inputs", es))
	got, err := FilesDigest("inputs", es)
	if err != nil || got != want {
		t.Fatalf("FilesDigest = %x, %v", got, err)
	}
	if r, _ := FilesDigest("reads", es); r == got {
		t.Fatal("section name must be part of the digest")
	}
	if _, err := FilesDigest("tools", es); err == nil {
		t.Fatal("unknown section accepted")
	}
	if _, err := FilesDigest("inputs", []FileEntry{es[1], es[0]}); err == nil {
		t.Fatal("unsorted accepted")
	}
	cs := []Consumed{{Target: "pkg:a", Phase: PhaseBuild, Key: d}}
	cd, err := ConsumedDigest(cs)
	if err != nil || cd != sha256.Sum256(putConsumed(nil, cs)) {
		t.Fatalf("ConsumedDigest = %x, %v", cd, err)
	}
	if _, err := ConsumedDigest([]Consumed{{Target: "x", Phase: "nope", Key: d}}); err == nil {
		t.Fatal("bad phase accepted")
	}
}

func TestToolsAndEnvDigests(t *testing.T) {
	ts := []Tool{{Name: "a", Version: "1", Kind: ToolHost, Identity: []byte("1")}, {Name: "b", Version: "image", Kind: ToolHost, Identity: []byte("img")}}
	td, err := ToolsDigest(ts)
	if err != nil || td != sha256.Sum256(putTools(nil, ts)) {
		t.Fatalf("ToolsDigest = %x, %v", td, err)
	}
	if _, err := ToolsDigest([]Tool{ts[1], ts[0]}); err == nil {
		t.Fatal("unsorted tools accepted")
	}
	es := []EnvVar{{Name: "A", Value: "1"}, {Name: "B", Value: ""}}
	ed, err := EnvDigest(es)
	if err != nil || ed != sha256.Sum256(putEnv(nil, es)) {
		t.Fatalf("EnvDigest = %x, %v", ed, err)
	}
	if ed == td {
		t.Fatal("sections must not collide")
	}
	if _, err := EnvDigest([]EnvVar{es[1], es[0]}); err == nil {
		t.Fatal("unsorted env accepted")
	}
	// The digests are the sections of the full preimage, byte for byte.
	pre, err := Preimage(Action{OS: "linux", Arch: "x64", Target: "t", Phase: PhaseTest, Tools: ts, Env: es})
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.HasSuffix(pre, append(putTools(nil, ts), putEnv(nil, es)...)) {
		t.Fatal("tools and env are not the tail of the preimage")
	}
}

package legacy

import (
	"bytes"
	"crypto/sha256"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"strings"
	"testing"
)

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

// scanReasonsOf returns the scan reasons (escape, indirect, unscanned) of a
// package in a fresh plan.
func scanReasonsOf(t *testing.T, f *fixture, name string) []string {
	t.Helper()
	r := f.plan(baseGraph()).rec(t, name)
	var out []string
	for _, why := range r.Reasons {
		if hasPrefixReason([]string{why}, "escape:") || hasPrefixReason([]string{why}, "indirect:") || hasPrefixReason([]string{why}, "unscanned:") {
			out = append(out, why)
		}
	}
	return out
}

func (f *fixture) symlink(rel, target string) {
	f.t.Helper()
	if runtime.GOOS == "windows" {
		f.t.Skip("symlinks need privileges on Windows")
	}
	full := filepath.Join(f.dir, filepath.FromSlash(rel))
	if err := os.MkdirAll(filepath.Dir(full), 0o755); err != nil {
		f.t.Fatal(err)
	}
	if err := os.Symlink(target, full); err != nil {
		f.t.Fatal(err)
	}
}

func (f *fixture) chmodExec(rel string) {
	f.t.Helper()
	if runtime.GOOS == "windows" {
		f.t.Skip("no executable bit on Windows")
	}
	if err := os.Chmod(filepath.Join(f.dir, filepath.FromSlash(rel)), 0o755); err != nil {
		f.t.Fatal(err)
	}
}

func wantReasons(t *testing.T, got []string, want ...string) {
	t.Helper()
	if !reflect.DeepEqual(got, want) && !(len(got) == 0 && len(want) == 0) {
		t.Fatalf("scan reasons\n got  %v\n want %v", got, want)
	}
}

const (
	pa = "code/packages/x/a"
	pz = "code/packages/x/z"
)

// ---------------------------------------------------------------------------
// plan-level behaviour
// ---------------------------------------------------------------------------

func TestScansReasonRetiredToolsEnvRemains(t *testing.T) {
	f := newFixture(t)
	f.write("code/kiln/legacy/pinned.json", `{"languages": ["x"]}`)
	p := f.plan(baseGraph())
	for _, r := range p.Packages {
		if contains(r.Reasons, "m1-incomplete:scans") {
			t.Fatalf("%s still carries m1-incomplete:scans", r.Name)
		}
		if !contains(r.Reasons, ReasonToolsEnv) {
			t.Fatalf("%s lost %s", r.Name, ReasonToolsEnv)
		}
		if r.Class == ClassCacheable {
			t.Fatalf("%s is cacheable while tools-env is outstanding", r.Name)
		}
	}
}

func TestEscapeInNonBuildTextFile(t *testing.T) {
	f := newFixture(t)
	f.write(pa+"/src.txt", "see ../z/src.txt for details\n")
	r := f.plan(baseGraph()).rec(t, "x/a")
	if r.Class != ClassUncacheable || !contains(r.Reasons, "escape:"+pz+"/src.txt") {
		t.Fatalf("%+v", r)
	}
	if r.Key == "" {
		t.Fatal("an escaping package must still get a key (measurement continues)")
	}
	// The package next door is unaffected.
	if got := scanReasonsOf(t, f, "x/z"); len(got) != 0 {
		t.Fatalf("leak: %v", got)
	}
}

func TestEscapeAllowedTargets(t *testing.T) {
	f := newFixture(t)
	for name, content := range map[string]string{
		"in package":           "../a/src.txt\n",
		"in package subdir":    "x/../y/z\n",
		"in closure dep":       "../c/src.txt\n",
		"in closure nested":    "../c/ext/n/src.txt\n",
		"in closure dev dep":   "../t/BUILD\n",
		"in closure enclosing": "../outer/Cargo.toml\n",
		"in read file":         "../../../shared/cfg.json\n",
		"in read dir":          "../../../grammars/g/sub/two.txt\n",
		"in read glob":         "../../../lang/x.mll\n",
		"quoted":               `cp "../c/src.txt" .` + "\n",
	} {
		f.write(pa+"/src.txt", content)
		if got := scanReasonsOf(t, f, "x/a"); len(got) != 0 {
			t.Errorf("%s: unexpected reasons %v", name, got)
		}
	}
}

func TestEscapeForbiddenTargets(t *testing.T) {
	f := newFixture(t)
	for name, tc := range map[string]struct{ content, want string }{
		"sibling outside closure": {"../z/src.txt\n", "escape:" + pz + "/src.txt"},
		"unlisted read":           {"../../../lang/other.txt\n", "escape:code/lang/other.txt"},
		"parent of the package":   {"../\n", "escape:code/packages/x"},
		"leaves the repository":   {strings.Repeat("../", 9) + "x\n", "escape:../../../../../x"},
		"trailing dotdot":         {"foo/../../z/..\n", "escape:code/packages/x"},
		"windows separator":       {`..\z\src.txt` + "\n", "escape:" + pz + "/src.txt"},
		"flag glued":              {"-I../z/inc\n", "escape:" + pz + "/inc"},
		"assignment":              {"--out=../z/o\n", "escape:" + pz + "/o"},
		"path list":               {"PATH=../c:../z/bin\n", "escape:" + pz + "/bin"},
		"variable prefix":         {"$ROOT/../c/x\n", "escape:$ROOT/../c/x"},
		"brace variable":          {"${ROOT}/..\n", "escape:${ROOT}/../"},
		"glob":                    {"../*/src.txt\n", "escape:../*/src.txt"},
		"file url":                {"file://../c/x\n", ""}, // absolute-rule territory, see below
	} {
		f.write(pa+"/src.txt", tc.content)
		got := scanReasonsOf(t, f, "x/a")
		if tc.want == "" {
			if !contains(got, "indirect:"+pa+"/src.txt") {
				t.Errorf("%s: %v", name, got)
			}
			continue
		}
		if !contains(got, tc.want) {
			t.Errorf("%s: want %q in %v", name, tc.want, got)
		}
	}
}

func TestDotDotInBuildFile(t *testing.T) {
	f := newFixture(t)
	f.write(pa+"/BUILD", "cd ../z && make\n")
	if got := scanReasonsOf(t, f, "x/a"); !contains(got, "escape:"+pz) {
		t.Fatal(got)
	}
	f.write(pa+"/BUILD", "cd ../c && make\n")
	wantReasons(t, scanReasonsOf(t, f, "x/a"))
}

func TestIndirectionInBuildAndOtherFiles(t *testing.T) {
	cases := []string{
		"git rev-parse --show-toplevel", "git   ls-files", "x=$(dirname $0)", "realpath .", "readlink -f x",
		"cd $(pwd)", "$(pwd -P)", "`pwd`", "echo $PWD", "echo ${PWD}", "echo $OLDPWD", "echo ${OLDPWD}",
		"echo $0", "echo ${0%/*}", "${BASH_SOURCE[0]}", "$BASH_SOURCE", "echo $HOME", "echo ${HOME}",
		"cp ~/x .", "cd ..", "cd ../", "cd .. && make", `cd ".."`, "(cd ..; make)", "cd ..\n", "cd..", "https://x/../y",
		"/usr/bin/foo", "/tmp", "/etc/passwd", "/opt/x", "//server/share", "/foo/bar", "#!/bin/bash", "-I/usr/include", "--prefix=/opt/x", "///etc/x",
		"%~dp0", "%CD%", "$PSScriptRoot", "const d = __dirname;",
	}
	for _, c := range cases {
		for _, file := range []string{"BUILD", "tool.sh"} {
			f := newFixture(t)
			f.write(pz+"/"+file, "echo start\n"+c+"\n")
			got := scanReasonsOf(t, f, "x/z")
			if !contains(got, "indirect:"+pz+"/"+file) {
				t.Errorf("%q in %s: not flagged (%v)", c, file, got)
			}
			if r := f.plan(baseGraph()).rec(t, "x/z"); r.Class != ClassUncacheable || r.Key == "" {
				t.Errorf("%q: %+v", c, r)
			}
		}
	}
}

func TestIndirectionNegatives(t *testing.T) {
	for _, c := range []string{
		"echo hi", "#!/bin/sh", "#!/usr/bin/env bash", "cmd >/dev/null 2>&1", "x=/dev/null",
		"// a comment", "//go:build linux", "/* block */", "a / b",
		"http://x/usr/bin", "/^re+$/", "cd ../x", "abcd ..", "git status", "git ls", "say $01",
		"wait.. what", "a..b", "x ... y", "/", "/// doc comment", "</ItemGroup>", "<p>x</p>", "cl /permissive- /W4", "echo \"$HOMEDIR\"x",
	} {
		// Note "$01" and "$HOMEDIR" DO match the substring markers `$0` and
		// `$HOME`: they are the documented false positives, asserted below.
		if strings.Contains(c, "$0") || strings.Contains(c, "$HOME") {
			if !hasMarker(c) {
				t.Errorf("%q should be a (false-positive) marker hit", c)
			}
			continue
		}
		fs := scanContent("a/b", c+"\n")
		if fs.indirect {
			t.Errorf("%q flagged as indirect", c)
		}
	}
}

func TestTokenisation(t *testing.T) {
	const dir = "a/b/c"
	for _, tc := range []struct {
		in   string
		refs []string
		dyn  []string
		ind  bool
	}{
		{in: "../../x", refs: []string{"a/x"}},
		{in: `..\x`, refs: []string{"a/b/x"}},
		{in: `"../x"`, refs: []string{"a/b/x"}},
		{in: `'../x'`, refs: []string{"a/b/x"}},
		{in: "--flag=../x", refs: []string{"a/b/x"}},
		{in: "-I../inc", refs: []string{"a/b/inc"}},
		{in: "$(cd ..)", refs: []string{"a/b"}, ind: true},
		{in: "$(cat ../x)", refs: []string{"a/b/x"}},
		{in: "PATH=../b:../c", refs: []string{"a/b/b", "a/b/c"}},
		{in: "f(../x,../y)", refs: []string{"a/b/x", "a/b/y"}},
		{in: "a;../x", refs: []string{"a/b/x"}},
		{in: "x>../out", refs: []string{"a/b/out"}},
		{in: "x|../tool", refs: []string{"a/b/tool"}},
		{in: "../", refs: []string{"a/b"}},
		{in: "a/..", refs: []string{"a/b/c"}},
		{in: "x/../y", refs: []string{"a/b/c/y"}},
		{in: strings.Repeat("../", 5) + "x", refs: []string{"../../x"}},
		{in: "https://x/../y", ind: true},
		{in: "see https://x/../y and ../z", refs: []string{"a/b/z"}, ind: true},
		{in: `"https://x/";cat ../z`, refs: []string{"a/b/z"}},
		{in: "file://../x", ind: true},
		{in: "wait..", refs: nil},
		{in: "a..b"},
		{in: "x ... y"},
		{in: "/a/../b", ind: true},
		{in: "$ROOT/../x", dyn: []string{"$ROOT/../x"}},
		{in: "%X%/../x", dyn: []string{"%X%/../x"}},
		{in: "~/../x", dyn: []string{"~/../x"}, ind: true},
		{in: "../../x ../../x", refs: []string{"a/x"}},
	} {
		got := scanContent(dir, tc.in)
		if !reflect.DeepEqual(got.refs, tc.refs) || !reflect.DeepEqual(got.dynamic, tc.dyn) || got.indirect != tc.ind {
			t.Errorf("%q\n got  refs=%v dyn=%v ind=%v\n want refs=%v dyn=%v ind=%v", tc.in, got.refs, got.dynamic, got.indirect, tc.refs, tc.dyn, tc.ind)
		}
	}
}

func TestAbsolutePathRule(t *testing.T) {
	for tok, want := range map[string]bool{
		"/dev/null": false, "/usr/bin/env": false, "/bin/sh": false,
		"/bin/bash": true, "/etc": true, "/tmp": true, "/etc/passwd": true, "/opt/x": true, "/foo/bar": true,
		"/Windows": true, "/__w": true, "/0": false, "/.git": false, "/_x": false, "/$X": false, "/W4": false,
		"/permissive-": false, "/p": false, "/ItemGroup": false, "/td": false, "/tmpx": false,
		"/": false, "//": false, "///": false, "////": false, "//TODO": false, "//go": false, "///doc": false,
		"///x": false, "///etc/x": true, "//server/share": true, "//a/../b": true, "/x/..": true, "/..": true,
		"/*": false, "/^x": false, "/(": false, "/{": false,
		"#!/bin/sh": false, "#!/bin/bash": true, "#!/usr/bin/env": false, "-I/usr/include": true, "-L/opt/lib": true,
		"-9/x": false, "-/x": false, "-W/p": false,
		"relative/x": false, "": false, "x": false,
	} {
		if got := absoluteOutside(tok); got != want {
			t.Errorf("absoluteOutside(%q) = %v, want %v", tok, got, want)
		}
	}
}

func TestReasonCapAndOrder(t *testing.T) {
	f := newFixture(t)
	for i := 0; i < 30; i++ {
		f.write(fmt.Sprintf("%s/m%02d.sh", pz, i), "echo $0\n")
	}
	got := scanReasonsOf(t, f, "x/z")
	if len(got) != maxScanReasons {
		t.Fatalf("%d reasons, want %d", len(got), maxScanReasons)
	}
	for i := 1; i < len(got); i++ {
		if got[i-1] >= got[i] {
			t.Fatalf("unsorted: %v", got)
		}
	}
	if got[0] != "indirect:"+pz+"/m00.sh" {
		t.Fatal(got[0])
	}
}

func TestReasonTextIsPrintable(t *testing.T) {
	if printable("a\x1b[31m\nb\x7f") != "a[31mb" {
		t.Fatal(printable("a\x1b[31m\nb\x7f"))
	}
}

func TestHistogramGroupsScanReasons(t *testing.T) {
	f := newFixture(t)
	f.write(pz+"/BUILD", "echo $0\n")
	f.write(pz+"/src.txt", "../a/x\n")
	f.write(pz+"/big.txt", strings.Repeat("a\n", maxScanBytes))
	p := f.plan(baseGraph())
	h := p.Histogram.Reasons
	if h["indirect"] != 1 || h["escape"] != 1 || h["unscanned"] != 1 {
		t.Fatalf("%v", h)
	}
	for k := range h {
		if strings.Contains(k, ":") && !strings.HasPrefix(k, "unpinned:") && !strings.HasPrefix(k, "m1-incomplete:") {
			t.Fatalf("ungrouped reason %q", k)
		}
	}
}

// ---------------------------------------------------------------------------
// text, size cap, executables
// ---------------------------------------------------------------------------

func TestOverCapTextIsUnscanned(t *testing.T) {
	f := newFixture(t)
	f.write(pz+"/at_cap.txt", strings.Repeat("a", maxScanBytes))
	wantReasons(t, scanReasonsOf(t, f, "x/z"))
	f.write(pz+"/big.txt", strings.Repeat("a", maxScanBytes+1))
	wantReasons(t, scanReasonsOf(t, f, "x/z"), "unscanned:"+pz+"/big.txt")
	r := f.plan(baseGraph()).rec(t, "x/z")
	if r.Class != ClassUncacheable || r.Key == "" {
		t.Fatalf("%+v", r)
	}
}

func TestExecutableBinaryIsUnscanned(t *testing.T) {
	f := newFixture(t)
	f.write(pz+"/prog", "\x7fELF\x00\x01\x02")
	f.write(pz+"/data.bin", "\x89PNG\r\n\x1a\n\x00")
	f.write(pz+"/script.sh", "#!/bin/sh\necho hi\n")
	f.write(pz+"/bigbin", strings.Repeat("\x00", 3*maxScanBytes))
	f.write(pz+"/bigdata", "\x89PNG\r\n\x1a\n"+strings.Repeat("\x00", 3*maxScanBytes))
	f.chmodExec("code/packages/x/z/prog")
	f.chmodExec("code/packages/x/z/script.sh")
	f.write(pz+"/blob", "\x01\x02\x00\x03") // non-text, not a known inert format, not executable
	f.write(pz+"/pngexec", "\x89PNG\r\n\x1a\n\x00")
	f.chmodExec("code/packages/x/z/bigbin")
	f.chmodExec("code/packages/x/z/pngexec")
	// data.bin and bigdata: inert image magic, not executable -> hashed, not scanned.
	// script.sh: executable TEXT -> scanned, clean.
	// blob: unknown binary -> unscanned even though it is not executable.
	// pngexec: inert magic but executable -> unscanned.
	wantReasons(t, scanReasonsOf(t, f, "x/z"), "unscanned:"+pz+"/bigbin", "unscanned:"+pz+"/blob",
		"unscanned:"+pz+"/pngexec", "unscanned:"+pz+"/prog")
}

func TestBinaryContentIsNotScannedForMarkers(t *testing.T) {
	f := newFixture(t)
	f.write(pz+"/img.bin", "\x89PNG\r\n\x1a\n\x00$0 ../../../etc/passwd /usr/bin/x dirname")
	wantReasons(t, scanReasonsOf(t, f, "x/z"))
}

func TestKindOf(t *testing.T) {
	webp := append([]byte("RIFF\x10\x00\x00\x00WEBP"), 0, 0)
	for name, tc := range map[string]struct {
		in   []byte
		want contentKind
	}{
		"empty":         {nil, kindText},
		"ascii":         {[]byte("hello\n"), kindText},
		"utf8":          {[]byte("héllo ✓"), kindText},
		"invalid utf8":  {[]byte("a\xffb\x80"), kindText}, // no NUL: still scanned as bytes
		"nul":           {[]byte("a\x00b"), kindBinary},
		"elf":           {[]byte("\x7fELF\x00\x01"), kindBinary},
		"utf16 le":      {[]byte("\xff\xfea\x00"), kindUTF16},
		"utf16 be":      {[]byte("\xfe\xff\x00a"), kindUTF16},
		"png":           {[]byte("\x89PNG\r\n\x1a\n\x00"), kindInert},
		"png lookalike": {[]byte("\x89PNG\x00\x00"), kindBinary},
		"jpeg":          {[]byte("\xff\xd8\xff\xe0\x00"), kindInert},
		"gif":           {[]byte("GIF89a\x00"), kindInert},
		"webp":          {webp, kindInert},
		"riff not webp": {[]byte("RIFF\x10\x00\x00\x00WAVE\x00"), kindBinary},
		"riff short":    {[]byte("RIFF\x00"), kindBinary},
		"ico":           {[]byte("\x00\x00\x01\x00\x01"), kindInert},
		"pdf":           {[]byte("%PDF-1.7\n\x00"), kindInert},
		"ttf":           {[]byte("\x00\x01\x00\x00\x00"), kindInert},
		"otf":           {[]byte("OTTO\x00"), kindInert},
		"woff":          {[]byte("wOFF\x00"), kindInert},
		"woff2":         {[]byte("wOF2\x00"), kindInert},
	} {
		if got := kindOf(tc.in); got != tc.want {
			t.Errorf("%s: kindOf = %v, want %v", name, got, tc.want)
		}
	}
}

func TestDecodeUTF16(t *testing.T) {
	if got := decodeUTF16([]byte("\xff\xfeh\x00i\x00\x3d\xd8\x00\xde")); got != "hi\U0001F600" {
		t.Fatalf("le: %q", got)
	}
	if got := decodeUTF16([]byte("\xfe\xff\x00h\x00i\x00")); got != "hi" { // odd trailing byte dropped
		t.Fatalf("be: %q", got)
	}
}

// The digest hashAndScan produces is exactly the digest of the whole content,
// whichever branch it takes.
func TestHashAndScanDigestMatches(t *testing.T) {
	for _, n := range []int{0, 1, headBytes - 1, headBytes, headBytes + 1, maxScanBytes, maxScanBytes + 1, 2*maxScanBytes + 17} {
		for _, fill := range []byte{'a', 0} {
			data := bytes.Repeat([]byte{fill}, n)
			h := sha256.New()
			if _, err := hashAndScan(bytes.NewReader(data), h, false, "d"); err != nil {
				t.Fatal(err)
			}
			if want := sha256.Sum256(data); !bytes.Equal(h.Sum(nil), want[:]) {
				t.Fatalf("n=%d fill=%d: digest differs", n, fill)
			}
		}
	}
}

type failingReader struct{ after int }

func (r *failingReader) Read(p []byte) (int, error) {
	if r.after <= 0 {
		return 0, fmt.Errorf("boom")
	}
	n := min(len(p), r.after)
	for i := 0; i < n; i++ {
		p[i] = 'a'
	}
	r.after -= n
	return n, nil
}

func TestHashAndScanReadErrors(t *testing.T) {
	for _, after := range []int{0, 100, headBytes + 50} {
		if _, err := hashAndScan(&failingReader{after: after}, sha256.New(), false, "d"); err == nil {
			t.Errorf("after=%d: no error", after)
		}
	}
	// A failing remainder of binary content and of over-cap text.
	if _, err := hashAndScan(&multiReader{parts: [][]byte{bytes.Repeat([]byte{0}, headBytes)}}, sha256.New(), false, "d"); err == nil {
		t.Error("binary remainder error not reported")
	}
	if _, err := hashAndScan(&multiReader{parts: [][]byte{bytes.Repeat([]byte{'a'}, maxScanBytes+10)}}, sha256.New(), false, "d"); err == nil {
		t.Error("over-cap remainder error not reported")
	}
}

// multiReader yields its parts and then fails.
type multiReader struct {
	parts [][]byte
}

func (r *multiReader) Read(p []byte) (int, error) {
	if len(r.parts) == 0 {
		return 0, fmt.Errorf("boom")
	}
	n := copy(p, r.parts[0])
	r.parts[0] = r.parts[0][n:]
	if len(r.parts[0]) == 0 {
		r.parts = r.parts[1:]
	}
	return n, nil
}

// ---------------------------------------------------------------------------
// symlinks
// ---------------------------------------------------------------------------

func TestSymlinkInsideClosureIsFine(t *testing.T) {
	f := newFixture(t)
	f.symlink(pa+"/to-dep", "../c/src.txt")
	f.symlink(pa+"/to-self", "src.txt")
	f.symlink(pa+"/dangling", "no-such-file")
	f.symlink(pa+"/to-read", "../../../shared/cfg.json")
	f.symlink(pa+"/to-read-dir", "../../../grammars/g/sub")
	f.symlink(pa+"/chain1", "chain2")
	f.symlink(pa+"/chain2", "../c/src.txt")
	f.symlink(pa+"/dir-link", "../c")
	f.symlink(pa+"/via-dir-link", "dir-link/src.txt")
	wantReasons(t, scanReasonsOf(t, f, "x/a"))
	// Links are part of the key: retargeting changes it.
	k1 := f.plan(baseGraph()).rec(t, "x/a").Key
	f.remove(pa + "/to-self")
	f.symlink(pa+"/to-self", "BUILD")
	if f.plan(baseGraph()).rec(t, "x/a").Key == k1 {
		t.Fatal("retargeting a link did not change the key")
	}
}

func TestSymlinkLeavingClosure(t *testing.T) {
	f := newFixture(t)
	f.symlink(pa+"/out", "../z/src.txt")
	wantReasons(t, scanReasonsOf(t, f, "x/a"), "escape:"+pa+"/out")
	r := f.plan(baseGraph()).rec(t, "x/a")
	if r.Class != ClassUncacheable || r.Key == "" {
		t.Fatalf("%+v", r)
	}
	// The same link is fine for a package that has z in its closure.
	g := baseGraph()
	g.Packages[0].Deps["linux"] = append(g.Packages[0].Deps["linux"], "x/z")
	if got := f.plan(g).rec(t, "x/a"); hasPrefixReason(got.Reasons, "escape:") {
		t.Fatalf("%v", got.Reasons)
	}
}

func TestSymlinkChains(t *testing.T) {
	f := newFixture(t)
	f.symlink(pa+"/l1", "l2")
	f.symlink(pa+"/l2", "../z/src.txt")
	wantReasons(t, scanReasonsOf(t, f, "x/a"), "escape:"+pa+"/l1", "escape:"+pa+"/l2")
}

func TestSymlinkLoop(t *testing.T) {
	f := newFixture(t)
	f.symlink(pa+"/l1", "l2")
	f.symlink(pa+"/l2", "l1")
	f.symlink(pa+"/self", "self")
	wantReasons(t, scanReasonsOf(t, f, "x/a"), "escape:"+pa+"/l1", "escape:"+pa+"/l2", "escape:"+pa+"/self")
}

func TestSymlinkHopLimit(t *testing.T) {
	chain := func(n int) []string {
		f := newFixture(t)
		for i := 0; i < n; i++ {
			next := fmt.Sprintf("h%02d", i+1)
			if i == n-1 {
				next = "src.txt"
			}
			f.symlink(fmt.Sprintf("%s/h%02d", pa, i), next)
		}
		return scanReasonsOf(t, f, "x/a")
	}
	if got := chain(maxLinkHops); len(got) != 0 {
		t.Fatalf("%d hops must be allowed: %v", maxLinkHops, got)
	}
	got := chain(maxLinkHops + 1)
	if !contains(got, "escape:"+pa+"/h00") {
		t.Fatalf("%d hops must be refused: %v", maxLinkHops+1, got)
	}
	if contains(got, "escape:"+pa+"/h01") {
		t.Fatalf("h01 needs only %d hops: %v", maxLinkHops, got)
	}
}

func TestSymlinkLeavingRepositoryOrAbsolute(t *testing.T) {
	f := newFixture(t)
	f.symlink(pa+"/up", strings.Repeat("../", 12)+"etc/passwd")
	f.symlink(pa+"/abs", "/etc/passwd")
	f.symlink(pa+"/drive", "C:/Windows/x")
	f.symlink(pa+"/dotdot-in-the-middle", "src.txt/../../z/BUILD")
	f.symlink(pa+"/file-as-dir", "src.txt/x")
	got := scanReasonsOf(t, f, "x/a")
	for _, l := range []string{"up", "abs", "drive", "dotdot-in-the-middle", "file-as-dir"} {
		if !contains(got, "escape:"+pa+"/"+l) {
			t.Errorf("%s: %v", l, got)
		}
	}
}

func TestSymlinkDirectoryLinks(t *testing.T) {
	f := newFixture(t)
	f.symlink(pa+"/dir-out", "../z")
	f.symlink(pa+"/dir-in", "../c")
	f.symlink(pa+"/via-out", "dir-out/src.txt") // resolves THROUGH a directory link
	f.symlink(pa+"/via-in", "dir-in/src.txt")
	f.symlink(pa+"/dot-slash", "./dir-in/./src.txt")
	got := scanReasonsOf(t, f, "x/a")
	want := []string{"escape:" + pa + "/dir-out", "escape:" + pa + "/via-out"}
	wantReasons(t, got, want...)
}

func TestSymlinkInReadsNotJustInPackage(t *testing.T) {
	f := newFixture(t)
	// x/c reads code/grammars/g (a directory); x/a has c in its closure.
	f.symlink("code/grammars/g/inside", "one.txt")           // inside the read dir
	f.symlink("code/grammars/g/outside", "../../lang/other") // outside everything
	for _, name := range []string{"x/c", "x/a"} {
		got := scanReasonsOf(t, f, name)
		wantReasons(t, got, "escape:code/grammars/g/outside")
	}
	// x/z does not read it.
	wantReasons(t, scanReasonsOf(t, f, "x/z"))
}

func TestSymlinkInGlobAndFileReads(t *testing.T) {
	f := newFixture(t)
	f.symlink("code/lang/l.mll", "../../README.md") // matched by x/b's glob; target is not a read
	wantReasons(t, scanReasonsOf(t, f, "x/b"), "escape:code/lang/l.mll")
	wantReasons(t, scanReasonsOf(t, f, "x/a"), "escape:code/lang/l.mll")
	// A read FILE that is itself a link (ancestor configuration).
	f2 := newFixture(t)
	f2.remove("code/packages/x/.npmrc")
	f2.symlink("code/packages/x/.npmrc", "../../../code/lang/other")
	wantReasons(t, scanReasonsOf(t, f2, "x/z"), "escape:code/packages/x/.npmrc")
}

func TestSymlinkedFileCannotHideAnEscape(t *testing.T) {
	// A symlink is never opened (Source.Open refuses it), so its target's
	// CONTENT is neither hashed nor scanned: the link target check is the rule.
	f := newFixture(t)
	f.write(pz+"/evil.sh", "cd .. && cat ../../../etc/passwd\n")
	f.symlink(pa+"/evil", "../z/evil.sh")
	got := scanReasonsOf(t, f, "x/a")
	wantReasons(t, got, "escape:"+pa+"/evil")
}

func TestLinkHelpers(t *testing.T) {
	for in, want := range map[string]bool{
		"": true, "/x": true, "C:/x": true, "c:x": true, "x": false, "../x": false, "a:b/c": true, "1:/x": false,
	} {
		if got := absoluteTarget(in); got != want {
			t.Errorf("absoluteTarget(%q) = %v, want %v", in, got, want)
		}
	}
	f := newFixture(t)
	pl, err := f.planner()
	if err != nil {
		t.Fatal(err)
	}
	if ok, err := pl.isLink("code/packages/x/a"); ok || err != nil {
		t.Fatalf("a directory is not a listed link: %v %v", ok, err)
	}
	if _, err := pl.entry("nope/file"); err == nil {
		t.Fatal("entry of an unknown file must fail")
	}
}

func TestAllowSet(t *testing.T) {
	f := newFixture(t)
	pl, err := f.planner()
	if err != nil {
		t.Fatal(err)
	}
	a := pl.newAllowSet([]int{0}, []readItem{{path: "code/shared/cfg.json"}, {glob: true, path: "code/**/?*.mll"}})
	for p, want := range map[string]bool{
		pa: true, pa + "/deep/x": true, "code/packages/x": false, "code/shared/cfg.json": true,
		"code/shared": false, "code/lang/x.mll": true, "code/lang/.mll": false,
		"..": false, "../x": false, ".": false,
	} {
		if got := a.ok(p); got != want {
			t.Errorf("ok(%q) = %v, want %v", p, got, want)
		}
	}
}

package legacy

// Regression tests for the findings of the security review of the scan rules.

import (
	"strings"
	"testing"
	"unicode/utf16"
)

// utf16le encodes s as BOM-prefixed UTF-16 little endian.
func utf16le(s string) string {
	var b strings.Builder
	b.WriteString("\xff\xfe")
	for _, u := range utf16.Encode([]rune(s)) {
		b.WriteByte(byte(u))
		b.WriteByte(byte(u >> 8))
	}
	return b.String()
}

// HIGH-1: location idioms of non-shell languages, including three real
// snippets from this repository.
func TestLocationIdiomMarkers(t *testing.T) {
	for _, c := range []string{
		// real files
		`    root = Path(__file__).parent.parent.parent`,
		`    require File.join(__dir__, "..", "lib", "x")`,
		`    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();`,
		// the rest of the list
		"x = __FILE__", "console.log(__filename)", "const u = import.meta.url", "os.getcwd()", "os.Getwd()",
		"process.cwd()", "Dir.pwd", "std::env::current_dir()", "filepath.Dir(f)", "filepath.Abs(f)",
		"runtime.Caller(0)", "$MyInvocation.MyCommand.Path", "$PSCommandPath", "Get-Location",
		"echo $GITHUB_WORKSPACE", "echo $RUNNER_TEMP", "dir=__dirname",
		// PowerShell variables are case-insensitive
		"$pwd", "$Pwd\\x", "$HOME", "$Home", "$home", "$PsScriptRoot", "$psscriptroot", "${Pwd}", "%Cd%",
		// working-directory changes
		"cd..", "pushd ..", "Set-Location ..", "Push-Location ..", "sl ..", "sl..", "chdir(x)", "os.Chdir(x)",
		"process.chdir('x')", "std::env::set_current_dir(p)", "chdir ..",
	} {
		if !hasMarker(c) {
			t.Errorf("%q not flagged", c)
		}
		f := newFixture(t)
		f.write(pz+"/tool.src", "line\n"+c+"\n")
		if got := scanReasonsOf(t, f, "x/z"); !contains(got, "indirect:"+pz+"/tool.src") {
			t.Errorf("%q: plan did not flag it: %v", c, got)
		}
	}
	// Not markers.
	for _, c := range []string{"slow ..", "mysl ..", "cdrom", "x = file", "dir", "Pwd"} {
		if hasMarker(c) {
			t.Errorf("%q flagged", c)
		}
	}
}

func TestFindMarkersAllAndFirst(t *testing.T) {
	s := "dirname $0 && git rev-parse HEAD; git ls-files; cd ..; $PWD"
	all := findMarkers(s, false)
	for _, want := range []string{"dirname", "$0", "git rev-parse", "git ls-files", "cd ..", "$pwd"} {
		if !contains(all, want) {
			t.Errorf("missing %q in %v", want, all)
		}
	}
	if first := findMarkers(s, true); len(first) != 1 {
		t.Fatalf("first=%v", first)
	}
	if findMarkers("echo ok", false) != nil {
		t.Fatal("clean text produced markers")
	}
}

// A token that is exactly "..", "../" or "..\" is the parent directory.
func TestBareDotDotTokens(t *testing.T) {
	for _, in := range []string{"..", "../", `..\`, `".."`, "'../'", "join(__x, \"..\")"} {
		got := scanContent("a/b/c", in)
		if len(got.refs) != 1 || got.refs[0] != "a/b" {
			t.Errorf("%q: refs=%v", in, got.refs)
		}
	}
	// Lua's concatenation operator becomes an accepted false positive.
	if got := scanContent("a/b/c", "x = a .. b"); len(got.refs) != 1 {
		t.Errorf("lua concat: %v", got.refs)
	}
	f := newFixture(t)
	f.write(pa+"/src.txt", `File.join(base, "..")`+"\n")
	if got := scanReasonsOf(t, f, "x/a"); !contains(got, "escape:code/packages/x") {
		t.Fatalf("%v", got)
	}
}

// HIGH-2: a dependency's findings reach its dependents.
func TestMemberResultsPropagate(t *testing.T) {
	const pc = "code/packages/x/c"
	// x/a -> x/b -> x/c.  x/z is unrelated.
	f := newFixture(t)
	f.write(pc+"/tool.sh", "dir=$(dirname $0)\n")
	for _, name := range []string{"x/c", "x/b", "x/a"} {
		if got := scanReasonsOf(t, f, name); !contains(got, "indirect:"+pc+"/tool.sh") {
			t.Errorf("%s: %v", name, got)
		}
	}
	if got := scanReasonsOf(t, f, "x/z"); len(got) != 0 {
		t.Fatalf("x/z: %v", got)
	}

	f = newFixture(t)
	f.write(pc+"/big.txt", strings.Repeat("a", maxScanBytes+1))
	if got := scanReasonsOf(t, f, "x/a"); !contains(got, "unscanned:"+pc+"/big.txt") {
		t.Fatalf("unscanned did not propagate: %v", got)
	}

	// A member's ../ ref is judged against the DEPENDENT's region: ../z is
	// outside for x/c and x/a alike; ../a is outside for x/c (a is not in its
	// closure) but inside x/a itself.
	f = newFixture(t)
	f.write(pc+"/src.txt", "../z/f ../a/g\n")
	for _, name := range []string{"x/c", "x/a"} {
		if got := scanReasonsOf(t, f, name); !contains(got, "escape:code/packages/x/z/f") {
			t.Errorf("%s: %v", name, got)
		}
	}
	if got := scanReasonsOf(t, f, "x/c"); !contains(got, "escape:code/packages/x/a/g") {
		t.Errorf("x/c must escape to a: %v", got)
	}
	if got := scanReasonsOf(t, f, "x/a"); contains(got, "escape:code/packages/x/a/g") {
		t.Errorf("x/a may point at itself: %v", got)
	}

	// A member's dynamic token is an escape for the dependent.
	f = newFixture(t)
	f.write(pc+"/src.txt", "$ROOT/../x\n")
	if got := scanReasonsOf(t, f, "x/a"); !contains(got, "escape:$ROOT/../x") {
		t.Fatalf("%v", got)
	}

	// A member's link that leaves the dependent's region.
	f = newFixture(t)
	f.symlink(pc+"/out", "../z/src.txt")
	if got := scanReasonsOf(t, f, "x/a"); !contains(got, "escape:"+pc+"/out") {
		t.Fatalf("%v", got)
	}
}

// A `../` target is walked through symlinks before the region check.
func TestDotDotTargetFollowsSymlinks(t *testing.T) {
	f := newFixture(t)
	f.symlink(pa+"/hop", "../z") // a directory link to a package outside the closure
	f.write(pa+"/src.txt", "hop/src.txt\n../a/hop/src.txt\n")
	// "hop/src.txt" has no ".." so it is not a ref; "../a/hop/src.txt" is, and
	// it lands inside x/a textually but resolves through hop into x/z.
	got := scanReasonsOf(t, f, "x/a")
	if !contains(got, "escape:"+pa+"/hop") || !contains(got, "escape:"+pa+"/hop/src.txt") {
		t.Fatalf("%v", got)
	}
	// Through a link that stays inside the closure it is fine.
	f = newFixture(t)
	f.symlink(pa+"/hop", "../c")
	f.write(pa+"/src.txt", "../a/hop/src.txt\n")
	wantReasons(t, scanReasonsOf(t, f, "x/a"))
}

// MEDIUM-3: text decoding.
func TestUTF16AndInvalidUTF8AreScanned(t *testing.T) {
	f := newFixture(t)
	f.write(pz+"/a.ps1", utf16le("Set-Location $PSScriptRoot\n"))
	f.write(pz+"/b.sh", "\xff echo $0\n") // not valid UTF-8, but no NUL: still text
	f.write(pz+"/c.ps1", "\xfe\xff\x00$\x00P\x00W\x00D\x00")
	f.write(pz+"/d.txt", utf16le("see ../a/x\n"))
	f.write(pz+"/e.txt", utf16le("fine\n"))
	got := scanReasonsOf(t, f, "x/z")
	for _, want := range []string{"indirect:" + pz + "/a.ps1", "indirect:" + pz + "/b.sh", "indirect:" + pz + "/c.ps1", "escape:code/packages/x/a/x"} {
		if !contains(got, want) {
			t.Errorf("missing %s in %v", want, got)
		}
	}
	if contains(got, "indirect:"+pz+"/e.txt") {
		t.Errorf("clean utf-16 flagged: %v", got)
	}
}

// MEDIUM-4: read files are scanned too.
func TestReadFilesAreScanned(t *testing.T) {
	// A boundary-input file read by x/a (code/shared/cfg.json).
	f := newFixture(t)
	f.write("code/shared/cfg.json", `{"p": "$HOME/x", "o": "../lang/other.txt"}`)
	got := scanReasonsOf(t, f, "x/a")
	if !contains(got, "indirect:code/shared/cfg.json") || !contains(got, "escape:code/lang/other.txt") {
		t.Fatalf("%v", got)
	}
	// ...but not by an unrelated package.
	if got := scanReasonsOf(t, f, "x/z"); len(got) != 0 {
		t.Fatalf("x/z: %v", got)
	}
	// A file under a read directory (x/c reads code/grammars/g; x/a has c).
	f = newFixture(t)
	f.write("code/grammars/g/sub/two.txt", "echo $0\n")
	f.write("code/grammars/g/big.bin", "\x00\x01")
	for _, name := range []string{"x/c", "x/a"} {
		got := scanReasonsOf(t, f, name)
		if !contains(got, "indirect:code/grammars/g/sub/two.txt") || !contains(got, "unscanned:code/grammars/g/big.bin") {
			t.Errorf("%s: %v", name, got)
		}
	}
	// A file matched by a read glob (x/b's code/**/?*.mll).
	f = newFixture(t)
	f.write("code/lang/x.mll", "dirname\n")
	if got := scanReasonsOf(t, f, "x/b"); !contains(got, "indirect:code/lang/x.mll") {
		t.Fatalf("%v", got)
	}
	// Ancestor configuration (mise.toml sits at the repository root).
	f = newFixture(t)
	f.write("mise.toml", "[env]\nHOME_X = \"$HOME\"\n")
	if got := scanReasonsOf(t, f, "x/z"); !contains(got, "indirect:mise.toml") {
		t.Fatalf("%v", got)
	}
}

// MEDIUM-5 is covered by TestLocationIdiomMarkers; MEDIUM-6 follows.
func TestAbsolutePathGaps(t *testing.T) {
	for _, c := range []string{
		"${TMPDIR:-/tmp}", "@/etc/passwd", "-isystem/usr/include", "[/etc", `\usr\bin\x`, `C:\Windows\x`, "C:/x/y",
		`\\host\share\x`, "//tmp", "~user/x", "~root", `cmd /c "D:\tool.exe"`, "(/etc/x)", "{/opt/x}", "+/usr/lib",
	} {
		if !scanContent("a/b", c+"\n").indirect {
			t.Errorf("%q not flagged", c)
		}
	}
	for _, c := range []string{
		"https://example.com/a/b", "http://x", "ab:/x", "~~strike~~", "~ ", "x::y", "${TMPDIR:-}", "-I", "-I/p", "[x]",
		`\n`, `\\`, `\\\`, `a\\b`,
	} {
		if scanContent("a/b", c+"\n").indirect {
			t.Errorf("%q flagged", c)
		}
	}
	if hasDrivePrefix("x:\\") || !hasDrivePrefix(" x:\\y") || hasDrivePrefix("ab:\\y") {
		t.Error("drive prefix rules")
	}
}

// LOW-8: a glob admits a listed file it matches, or a directory only when every
// file under it matches.
func TestGlobAdmitsOnlyMatchingTargets(t *testing.T) {
	f := newFixture(t)
	f.write("code/mll/a.mll", "x")
	f.write("code/mll/b.mll", "x")
	f.write("code/mix/a.mll", "x")
	f.write("code/mix/c.txt", "x")
	pl, err := f.planner()
	if err != nil {
		t.Fatal(err)
	}
	a := pl.newAllowSet(nil, []readItem{{glob: true, path: "code/**/?*.mll"}})
	for target, want := range map[string]bool{
		"code/mll/a.mll": true, // a listed matching file
		"code/mll":       true, // every file under it matches
		"code/mix":       false,
		"code/mix/a.mll": true,
		"code/mix/c.txt": false,
		"code/nothing":   false, // not a file, and nothing under it
		"code/lang":      false, // contains `.mll`, which `?*` does not match
	} {
		if got := a.ok(target); got != want {
			t.Errorf("ok(%q) = %v, want %v", target, got, want)
		}
	}
}

// LOW-9 and LOW-10: link targets.
func TestLinkTargetRules(t *testing.T) {
	f := newFixture(t)
	f.symlink(pa+"/back", `..\z\src.txt`)
	f.symlink(pa+"/ok", "src.txt")
	got := scanReasonsOf(t, f, "x/a")
	wantReasons(t, got, "escape:"+pa+"/back")

	pl, err := f.planner()
	if err != nil {
		t.Fatal(err)
	}
	r, err := pl.entryFull(pa + "/ok")
	if err != nil || r.target != "src.txt" {
		t.Fatalf("%+v %v", r, err)
	}
	if tg, err := pl.target(pa + "/ok"); tg != "src.txt" || err != nil {
		t.Fatalf("%q %v", tg, err)
	}
}

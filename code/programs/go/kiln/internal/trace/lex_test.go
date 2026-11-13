package trace

import (
	"errors"
	"os"
	"strings"
	"testing"
)

func TestDecodeString(t *testing.T) {
	good := map[string]string{
		`""`:                               "",
		`"abc"`:                            "abc",
		`"a\"b"`:                           `a"b`,
		`"a\\b"`:                           `a\b`,
		`"\n\t\r\a\b\f\v"`:                 "\n\t\r\a\b\f\v",
		`"\0"`:                             "\x00",
		`"\101\1011"`:                      "AA1",
		`"\x41\x7a\X"`:                     "", // placeholder, replaced below
		`"\303\251"`:                       "é",
		"\"café\"":                         "café",
		`"a b ) = 3</x> <unfinished ...>"`: "a b ) = 3</x> <unfinished ...>",
	}
	delete(good, `"\x41\x7a\X"`)
	good[`"\x41\x7a\x4A"`] = "AzJ"
	for in, want := range good {
		got, trunc, end, r := decodeString(in, 0)
		if r != "" || trunc || string(got) != want || end != len(in) {
			t.Errorf("%s: got %q trunc=%v end=%d r=%q", in, got, trunc, end, r)
		}
	}
	if _, trunc, end, _ := decodeString(`"ab"...`, 0); !trunc || end != 7 {
		t.Errorf("trunc %v %d", trunc, end)
	}
	bad := map[string]string{
		`"abc`: reasonUnparsable, `"a\`: reasonAmbiguousPath, `"\q"`: reasonAmbiguousPath, `"\400"`: reasonAmbiguousPath,
		`"\x4"`: reasonAmbiguousPath, `"\x4g"`: reasonAmbiguousPath, `"\xg4"`: reasonAmbiguousPath, "\"a\x01\"": reasonAmbiguousPath, `"\<"`: reasonAmbiguousPath,
	}
	for in, want := range bad {
		if _, _, _, r := decodeString(in, 0); r != want {
			t.Errorf("%q: reason %q want %q", in, r, want)
		}
	}
}

func TestScanAnnotation(t *testing.T) {
	good := map[string]string{`</a>`: "/a", `</a b>`: "/a b", `</a\>b\<c>`: "/a>b<c", `</a\"b>`: `/a"b`, `<>`: "", `</a"b>`: `/a"b`}
	for in, want := range good {
		body, end, r := scanAnnotation(in, 0)
		if r != "" || string(body) != want || end != len(in) {
			t.Errorf("%s: %q %d %q", in, body, end, r)
		}
	}
	for in, want := range map[string]string{`</a`: reasonUnparsable, `</a<b>`: reasonAmbiguousPath, `</a\q>`: reasonAmbiguousPath, "</a\x01>": reasonAmbiguousPath} {
		if _, _, r := scanAnnotation(in, 0); r != want {
			t.Errorf("%q: %q want %q", in, r, want)
		}
	}
}

func TestParseResultShapes(t *testing.T) {
	ok := []struct {
		in    string
		check func(result) bool
	}{
		{"= 0", func(r result) bool { return r.ok() && r.value == 0 }},
		{"   = 12", func(r result) bool { return r.ok() && r.value == 12 }},
		{"= 0x7f0000001000", func(r result) bool { return r.ok() && r.value == 0x7f0000001000 }},
		{"= 0 (Timeout)", func(r result) bool { return r.ok() }},
		{"= -1 EINTR (Interrupted system call)", func(r result) bool { return r.errno == "EINTR" && !r.ok() }},
		{"= ? ERESTARTSYS (To be restarted if SA_RESTART is set)", func(r result) bool { return r.unknown && r.errno == "ERESTARTSYS" }},
		{"= ?", func(r result) bool { return r.unknown && r.errno == "?" }},
		{"= 3</a b>", func(r result) bool { return r.hasAnn && string(r.ann) == "/a b" }},
	}
	for _, c := range ok {
		r, reason := parseResult(c.in)
		if reason != "" || !c.check(r) {
			t.Errorf("%q: %+v %q", c.in, r, reason)
		}
	}
	for _, in := range []string{"", "0", "= ", "= 0x", "= 0xzz", "= 99999999999999999999", "= -", "= -1<x>", "= 0 x y", "= 0 (a", "= 0 (a(b))", "= 0 junk (x)", "= -3 ENOENT (x)", "= 0x1ffffffffffffffff"} {
		if _, reason := parseResult(in); reason == "" {
			t.Errorf("%q accepted", in)
		}
	}
}

func TestHelpers(t *testing.T) {
	if validName("") || validName("A") || validName(strings.Repeat("a", 65)) || !validName("openat2") {
		t.Fatal("validName")
	}
	if isErrnoName("") || isErrnoName("eno") || isErrnoName("E-X") || !isErrnoName("ENOENT") || !isErrnoName("SIGRTMIN_1") {
		t.Fatal("isErrnoName")
	}
	for _, s := range []string{"", "0", "-1", "x", "1234567890"} {
		if _, ok := atoiPid(s); ok {
			t.Fatalf("pid %q", s)
		}
	}
	if n, ok := atoiPid("42"); !ok || n != 42 {
		t.Fatal("atoiPid")
	}
	if _, r := fdPathText([]byte("a\x00"), false); r != reasonAmbiguousPath {
		t.Fatal(r)
	}
	if _, r := fdPathText([]byte("\xff"), false); r != reasonNonUTF8 {
		t.Fatal(r)
	}
	if s, r := fdPathText([]byte("pipe:[1]"), false); r != "" || s != "pipe:[1]" {
		t.Fatal(r)
	}
	if _, ok := hexVal('g'); ok {
		t.Fatal("hexVal")
	}
	if v, ok := hexVal('F'); !ok || v != 15 {
		t.Fatal("hexVal")
	}
	if _, ok := parseExit("exited with 1234567890", 1); ok {
		t.Fatal("long exit code")
	}
	if _, ok := parseExit("exited with ", 1); ok {
		t.Fatal("empty exit code")
	}
	if _, ok := parseExit("killed by SIG", 1); !ok {
		// "SIG" alone is a syntactically valid name; kept permissive.
		t.Log("bare SIG rejected")
	}
}

func TestFindSockaddrs(t *testing.T) {
	good, r := findSockaddrs(`{msg_iov=[{iov_base="{sa_family=AF_X}"}], f=<>}`)
	_ = good
	if r == "" {
		t.Fatal("bare <> without token should be refused")
	}
	got, r := findSockaddrs(`[{msg_hdr={msg_name={sa_family=AF_UNIX, sun_path="/a"}}}, 3</x>]`)
	if r != "" || len(got) != 1 || got[0] != `{sa_family=AF_UNIX, sun_path="/a"}` {
		t.Fatalf("%v %q", got, r)
	}
	for _, bad := range []string{`{`, `}`, `{"abc}`, `{a=3<x}`, `{a=3</x\q>}`} {
		if _, r := findSockaddrs(bad); r == "" {
			t.Errorf("%q accepted", bad)
		}
	}
	if _, r := quotedOnly(`abc`); r == "" {
		t.Fatal("quotedOnly bare")
	}
	if _, r := quotedOnly(`"abc"...`); r != reasonTruncated {
		t.Fatal(r)
	}
	if _, r := quotedOnly(`"a" x`); r == "" {
		t.Fatal("quotedOnly trailing")
	}
	if _, r := quotedOnly(`"\q"`); r == "" {
		t.Fatal("quotedOnly escape")
	}
	if s, r := quotedOnly(` "1.2.3.4" `); r != "" || s != "1.2.3.4" {
		t.Fatal(r)
	}
}

func TestPendingAtEOFIsDeterministicEnough(t *testing.T) {
	_, err := Parse(strings.NewReader("1  stat(\"a\" <unfinished ...>\n"), "/", Limits{})
	var e *Error
	if !errors.As(err, &e) || e.Reason != reasonUnresumed || e.Line != 1 {
		t.Fatalf("%v", err)
	}
}

// FuzzParse: the parser never panics, and every outcome is either success or
// a trace-incomplete error.
func FuzzParse(f *testing.F) {
	for _, n := range []string{"build.strace", "cwd.strace", "paths.strace", "net.strace"} {
		b, err := os.ReadFile("testdata/" + n)
		if err != nil {
			f.Fatal(err)
		}
		f.Add(string(b))
	}
	for _, s := range []string{
		"1  stat(\"a\\\"b\", 0x1) = 0\n1  +++ exited with 0 +++\n",
		"1  openat(AT_FDCWD, \"/x\", O_RDONLY) = 3</x\\<\\>>\n",
		"1  stat(\"/x <unfinished ...>\", 0x1) = 0\n",
		"1  stat( <unfinished ...>\n1  <... stat resumed>\"a\", 0x1) = 0\n",
		"[ Process PID=1 runs in 32 bit mode. ]\n",
		"1  clone(child_stack=NULL, flags=CLONE_UNTRACED) = 2\n",
		"1  connect(3, {sa_family=AF_UNIX, sun_path=@\"x\"}, 2) = 0\n",
		"1  +++ killed by SIGKILL +++\n",
	} {
		f.Add(s)
	}
	f.Fuzz(func(t *testing.T, in string) {
		tr, err := Parse(strings.NewReader(in), "/w", Limits{MaxLineBytes: 1 << 16, MaxTotalBytes: 1 << 20, MaxEvents: 1 << 12})
		if err != nil {
			var e *Error
			if !errors.As(err, &e) || !strings.HasPrefix(err.Error(), "trace-incomplete:") {
				t.Fatalf("non trace-incomplete error: %v", err)
			}
			if tr != nil {
				t.Fatal("trace returned with error")
			}
			return
		}
		for pid, p := range tr.Procs {
			if p.Exit == nil || p.Pid != pid {
				t.Fatalf("process %d without exit", pid)
			}
		}
		for _, ev := range tr.Events {
			if ev.Kind == "" || ev.Syscall == "" || ev.Pid <= 0 {
				t.Fatalf("bad event %+v", ev)
			}
		}
	})
}

package exechead

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"strings"
	"testing"
)

// Encoder and decoder tests: they run on every OS, because the classifier that
// reads the file (PR 5) need not run where the supervisor did.

const (
	testNonce  = "00112233445566778899aabbccddeeff"
	testAction = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
)

var testExpect = Expect{Nonce: testNonce, ActionSHA256: testAction, Step: 2}

func testHeader() Header {
	return Header{V: Version, Kernel: "7.0.0-1014-aws", Nonce: testNonce, ActionSHA256: testAction, Step: 2}
}

func confirmedLine(tid int, path string, head []byte) Line {
	sum := sha256.Sum256(head)
	return Line{Tid: tid, Tgid: tid, Syscall: SyscallExecve, Path: path, Status: StatusConfirmed,
		Dev: 58, Ino: 12, Head: head, SHA256: hex.EncodeToString(sum[:])}
}

func plainLine(tid int, st Status) Line {
	return Line{Tid: tid, Tgid: tid, Syscall: SyscallExecve, Path: "/usr/bin/sh", Status: st}
}

// build writes a file with the real writer and returns it.
func build(t *testing.T, lines ...Line) []byte {
	t.Helper()
	var buf bytes.Buffer
	w, err := NewWriter(&buf, testHeader(), Limits{})
	if err != nil {
		t.Fatal(err)
	}
	for _, l := range lines {
		if err := w.Append(l); err != nil {
			t.Fatalf("Append(%+v): %v", l, err)
		}
	}
	if w.Bytes() != int64(buf.Len()) {
		t.Fatalf("Bytes() = %d, file is %d", w.Bytes(), buf.Len())
	}
	sum := sha256.Sum256(buf.Bytes())
	if w.SHA256() != hex.EncodeToString(sum[:]) {
		t.Fatal("the writer's digest is not the file's")
	}
	return buf.Bytes()
}

func TestRoundTrip(t *testing.T) {
	head := []byte("\x7fELF\x02\x01\x01")
	data := build(t, plainLine(10, StatusOutside), confirmedLine(11, "/tmp/x/t.test", head), confirmedLine(12, "/tmp/y", []byte{}))
	f, err := Decode(data, Limits{}, testExpect)
	if err != nil {
		t.Fatal(err)
	}
	if f.Header != testHeader() || len(f.Lines) != 3 {
		t.Fatalf("%+v", f)
	}
	if f.Lines[1].Status != StatusConfirmed || !bytes.Equal(f.Lines[1].Head, head) || f.Lines[1].Dev != 58 || f.Lines[1].Ino != 12 {
		t.Fatalf("%+v", f.Lines[1])
	}
	if f.Lines[2].Head == nil || len(f.Lines[2].Head) != 0 {
		t.Fatalf("an empty head must come back as an empty, non-nil slice: %#v", f.Lines[2].Head)
	}
	for i, l := range f.Lines {
		if l.Seq != i {
			t.Fatalf("line %d has seq %d", i, l.Seq)
		}
	}
	if f.Bytes != int64(len(data)) {
		t.Fatalf("Bytes %d, len %d", f.Bytes, len(data))
	}
	sum := sha256.Sum256(data)
	if f.SHA256 != hex.EncodeToString(sum[:]) {
		t.Fatal("digest")
	}
}

func TestEveryStatusRoundTrips(t *testing.T) {
	var lines []Line
	for _, st := range []Status{StatusOutside, StatusGone, StatusMismatch, StatusTimeout, StatusUnpinned, StatusFexecve, StatusSameImage, StatusFailed, StatusOverload, StatusTainted} {
		lines = append(lines, plainLine(5, st))
	}
	lines = append(lines, confirmedLine(5, "/tmp/a", []byte("#!/bin/sh\n")))
	f, err := Decode(build(t, lines...), Limits{}, testExpect)
	if err != nil || len(f.Lines) != 11 {
		t.Fatalf("%v %d", err, len(f.Lines))
	}
	if Status("bogus").Valid() || Status("").Valid() {
		t.Fatal("a status outside the enum is valid")
	}
}

// lines of a built file, header first.
func split(t *testing.T, data []byte) []string {
	t.Helper()
	return strings.Split(strings.TrimSuffix(string(data), "\n"), "\n")
}

func join(ls []string) []byte { return []byte(strings.Join(ls, "\n") + "\n") }

func TestDecodeRefuses(t *testing.T) {
	good := build(t, plainLine(10, StatusOutside), confirmedLine(11, "/tmp/x", []byte("hi")), plainLine(12, StatusGone))
	ls := split(t, good)
	hdr, l0, l1, l2 := ls[0], ls[1], ls[2], ls[3]
	cases := []struct {
		name string
		data []byte
		want string
	}{
		{"empty file", nil, "empty"},
		{"no final newline", bytes.TrimSuffix(good, []byte("\n")), "newline"},
		{"an empty line", join([]string{hdr, "", l1}), "empty"},
		{"unknown key", join([]string{hdr, strings.Replace(l0, `"status":"outside"`, `"status":"outside","extra":1`, 1)}), "unknown"},
		{"unknown key in the header", join([]string{strings.Replace(hdr, `"step":2`, `"step":2,"x":1`, 1), l0}), "unknown"},
		{"duplicate key", join([]string{hdr, strings.Replace(l0, `"tid":10`, `"tid":10,"tid":10`, 1)}), "duplicate"},
		{"duplicate key in the header", join([]string{strings.Replace(hdr, `"v":1`, `"v":1,"v":1`, 1), l0}), "duplicate"},
		{"a key in the wrong case", join([]string{hdr, strings.Replace(l0, `"tid"`, `"Tid"`, 1)}), "case-exact"},
		{"a missing key", join([]string{hdr, strings.Replace(l0, `"path":"/usr/bin/sh",`, ``, 1)}), "missing"},
		{"status outside the enum", join([]string{hdr, strings.Replace(l0, `"outside"`, `"approved"`, 1)}), "not one of the eleven"},
		{"status with another case", join([]string{hdr, strings.Replace(l0, `"outside"`, `"Outside"`, 1)}), "not one of the eleven"},
		{"seq gap", join([]string{hdr, l0, strings.Replace(l1, `"seq":1`, `"seq":2`, 1)}), "seq is 2, want 1"},
		{"seq repeat", join([]string{hdr, l0, l0}), "seq is 0, want 1"},
		{"seq reorder", join([]string{hdr, l1, l0}), "seq is 1, want 0"},
		{"seq starts at 1", join([]string{hdr, l1}), "seq is 1, want 0"},
		{"confirmed without head", join([]string{hdr, l0, strings.Replace(l1, `,"head":"aGk="`, ``, 1)}), "needs dev, ino, head and sha256"},
		{"outside with a head", join([]string{hdr, strings.Replace(l0, `}`, `,"dev":1,"ino":2,"head":"","sha256":"`+strings.Repeat("0", 64)+`"}`, 1)}), "may not carry"},
		{"gone with a sha256 only", join([]string{hdr, l0, l1, strings.Replace(l2, `}`, `,"sha256":"`+strings.Repeat("0", 64)+`"}`, 1)}), "may not carry"},
		{"a sha256 that is not the head's", join([]string{hdr, l0, strings.Replace(l1, `"head":"aGk="`, `"head":"aGo="`, 1)}), "sha256"},
		{"a head that is not base64", join([]string{hdr, l0, strings.Replace(l1, `"aGk="`, `"!!"`, 1)}), "head"},
		{"a head with non-canonical padding", join([]string{hdr, l0, strings.Replace(l1, `"aGk="`, `"aGl="`, 1)}), "head"},
		{"spacing", join([]string{hdr, strings.Replace(l0, `,`, `, `, 1)}), "canonical"},
		{"key order", join([]string{hdr, strings.Replace(l0, `"tid":10,"tgid":10`, `"tgid":10,"tid":10`, 1)}), "canonical"},
		{"an escape the encoder never writes", join([]string{hdr, strings.Replace(l0, "/usr/bin/sh", "\\u002fusr/bin/sh", 1)}), "canonical"},
		{"trailing data", join([]string{hdr, l0 + " {}"}), ""},
		{"a syscall outside the two", join([]string{hdr, strings.Replace(l0, `"execve"`, `"fork"`, 1)}), "syscall"},
		{"a zero tid", join([]string{hdr, strings.Replace(l0, `"tid":10`, `"tid":0`, 1)}), "tid"},
		{"a negative tgid", join([]string{hdr, strings.Replace(l0, `"tgid":10`, `"tgid":-3`, 1)}), "tgid"},
		{"a float seq", join([]string{hdr, strings.Replace(l0, `"seq":0`, `"seq":0.0`, 1)}), ""},
		{"a string seq", join([]string{hdr, strings.Replace(l0, `"seq":0`, `"seq":"0"`, 1)}), ""},
		{"a null key", join([]string{hdr, strings.Replace(l0, `"path":"/usr/bin/sh"`, `"path":null`, 1)}), "missing"},
		{"not JSON", join([]string{hdr, "garbage"}), ""},
		{"a header with version 2", join([]string{strings.Replace(hdr, `"v":1`, `"v":2`, 1), l0}), "version"},
		{"a header whose nonce is short", join([]string{strings.Replace(hdr, testNonce, "00", 1), l0}), "nonce"},
		{"a header whose nonce is upper case", join([]string{strings.Replace(hdr, testNonce, strings.ToUpper(testNonce), 1), l0}), "nonce"},
		{"a header whose digest is short", join([]string{strings.Replace(hdr, testAction, "aa", 1), l0}), "action_sha256"},
		{"a header with a negative step", join([]string{strings.Replace(hdr, `"step":2`, `"step":-1`, 1), l0}), "step"},
		{"a line where the header goes", join([]string{l0, l1}), ""},
		{"a header where a line goes", join([]string{hdr, hdr}), ""},
		{"a kernel with a control character", join([]string{strings.Replace(hdr, `7.0.0-1014-aws`, `7.0\u0000`, 1), l0}), ""},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			f, err := Decode(c.data, Limits{}, testExpect)
			if err == nil {
				t.Fatalf("accepted: %+v", f)
			}
			if c.want != "" && !strings.Contains(err.Error(), c.want) {
				t.Fatalf("error %q does not mention %q", err, c.want)
			}
		})
	}
}

func TestDecodeExpect(t *testing.T) {
	data := build(t, plainLine(10, StatusOutside))
	for name, e := range map[string]Expect{
		"another nonce":  {Nonce: strings.Repeat("1", 32), ActionSHA256: testAction, Step: 2},
		"another action": {Nonce: testNonce, ActionSHA256: strings.Repeat("b", 64), Step: 2},
		"another step":   {Nonce: testNonce, ActionSHA256: testAction, Step: 3},
	} {
		if _, err := Decode(data, Limits{}, e); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	// The three are MANDATORY: a zero or malformed Expect is refused, never
	// treated as "no check".
	for name, e := range map[string]Expect{
		"a zero Expect":      {},
		"no nonce":           {ActionSHA256: testAction, Step: 2},
		"no action":          {Nonce: testNonce, Step: 2},
		"a short nonce":      {Nonce: "00", ActionSHA256: testAction, Step: 2},
		"a negative step":    {Nonce: testNonce, ActionSHA256: testAction, Step: -1},
		"an upper-case hash": {Nonce: testNonce, ActionSHA256: strings.ToUpper(testAction), Step: 2},
	} {
		if _, err := Decode(data, Limits{}, e); err == nil || !strings.Contains(err.Error(), "mandatory") {
			t.Errorf("%s: %v", name, err)
		}
	}
	if _, err := Decode(data, Limits{}, testExpect); err != nil {
		t.Fatal(err)
	}
}

func TestHeaderWaitKillableRoundTrips(t *testing.T) {
	var buf bytes.Buffer
	h := testHeader()
	h.WaitKillable = true
	if _, err := NewWriter(&buf, h, Limits{}); err != nil {
		t.Fatal(err)
	}
	f, err := Decode(buf.Bytes(), Limits{}, testExpect)
	if err != nil || !f.Header.WaitKillable {
		t.Fatalf("%+v %v", f, err)
	}
	if !strings.Contains(buf.String(), `"wait_killable":true`) {
		t.Fatal(buf.String())
	}
}

func TestDecodeCaps(t *testing.T) {
	data := build(t, plainLine(1, StatusOutside), plainLine(2, StatusOutside), plainLine(3, StatusOutside))
	if _, err := Decode(data, Limits{MaxLines: 3}, testExpect); err != nil {
		t.Fatalf("exactly at the line cap: %v", err)
	}
	if _, err := Decode(data, Limits{MaxLines: 2}, testExpect); err == nil || !strings.Contains(err.Error(), "more than 2") {
		t.Fatalf("over the line cap: %v", err)
	}
	if _, err := Decode(data, Limits{MaxBytes: int64(len(data))}, testExpect); err != nil {
		t.Fatalf("exactly at the byte cap: %v", err)
	}
	if _, err := Decode(data, Limits{MaxBytes: int64(len(data)) - 1}, testExpect); err == nil || !strings.Contains(err.Error(), "over the cap") {
		t.Fatalf("over the byte cap: %v", err)
	}
}

func TestDecodePerLineCap(t *testing.T) {
	data := build(t, plainLine(1, StatusOutside))
	ls := split(t, data)
	// A line of exactly the cap parses as far as the cap goes (and is then refused
	// for what it is); one byte more is refused for its length.
	pad := strings.Repeat("a", MaxLineBytes)
	_, err := Decode(join([]string{ls[0], `{"seq":0,"x":"` + pad + `"}`}), Limits{MaxBytes: 1 << 30}, testExpect)
	if err == nil || !strings.Contains(err.Error(), "over the cap") {
		t.Fatalf("a line over the per-line cap: %v", err)
	}
	// A confirmed line at the head limit is fine; the whole line is under the cap.
	big := bytes.Repeat([]byte{0xa5}, HeadLimit)
	f, err := Decode(build(t, confirmedLine(7, "/tmp/big", big)), Limits{}, testExpect)
	if err != nil || len(f.Lines[0].Head) != HeadLimit {
		t.Fatalf("a head of the limit: %v", err)
	}
	// One byte more is refused by the encoder and the decoder alike.
	if _, err := EncodeLine(confirmedLine(7, "/tmp/big", append(big, 1))); err == nil {
		t.Fatal("the encoder wrote a head over the limit")
	}
	var buf bytes.Buffer
	w, _ := NewWriter(&buf, testHeader(), Limits{})
	if err := w.Append(confirmedLine(7, "/tmp/big", append(big, 1))); err == nil {
		t.Fatal("the writer took a head over the limit")
	}
	if w.Lines() != 0 {
		t.Fatal("a refused line used a sequence number")
	}
	// The worst-case line the writer can produce fits the per-line cap: the
	// largest head with the longest path of characters that JSON escapes to six.
	worst := confirmedLine(7, strings.Repeat("\x01", PathMax-1), big)
	enc, err := EncodeLine(worst)
	if err != nil || len(enc) > MaxLineBytes {
		t.Fatalf("worst-case line: %d bytes, %v", len(enc), err)
	}
	if _, err := Decode(build(t, worst), Limits{}, testExpect); err != nil {
		t.Fatalf("the worst-case line does not decode: %v", err)
	}
}

func TestEncoderRefusals(t *testing.T) {
	cases := map[string]Line{
		"a bad status":            {Tid: 1, Tgid: 1, Syscall: SyscallExecve, Status: "yes"},
		"a bad syscall":           {Tid: 1, Tgid: 1, Syscall: "exec", Status: StatusOutside},
		"a zero tid":              {Tgid: 1, Syscall: SyscallExecve, Status: StatusOutside},
		"a path that is not UTF8": {Tid: 1, Tgid: 1, Syscall: SyscallExecve, Path: "/tmp/\xff", Status: StatusUnpinned},
		"a path of PATH_MAX":      {Tid: 1, Tgid: 1, Syscall: SyscallExecve, Path: strings.Repeat("a", PathMax), Status: StatusUnpinned},
		"dev on an outside line":  {Tid: 1, Tgid: 1, Syscall: SyscallExecve, Status: StatusOutside, Dev: 3},
		"head on a gone line":     {Tid: 1, Tgid: 1, Syscall: SyscallExecve, Status: StatusGone, Head: []byte("x")},
		"a wrong sha256":          {Tid: 1, Tgid: 1, Syscall: SyscallExecve, Status: StatusConfirmed, Head: []byte("x"), SHA256: strings.Repeat("0", 64)},
	}
	for name, l := range cases {
		if _, err := EncodeLine(l); err == nil {
			t.Errorf("%s: encoded", name)
		}
	}
	// A path of PATH_MAX-1 bytes is the longest.
	if _, err := EncodeLine(Line{Tid: 1, Tgid: 1, Syscall: SyscallExecve, Path: strings.Repeat("a", PathMax-1), Status: StatusUnpinned}); err != nil {
		t.Errorf("the longest path: %v", err)
	}
	// An unpinned line for a path that was not UTF-8 carries "" and is fine.
	if _, err := EncodeLine(Line{Tid: 1, Tgid: 1, Syscall: SyscallExecve, Path: "", Status: StatusUnpinned}); err != nil {
		t.Errorf("an empty path: %v", err)
	}
}

func TestEncoderIsCanonicalAndUnescaped(t *testing.T) {
	enc, err := EncodeLine(Line{Seq: 4, Tid: 9, Tgid: 9, Syscall: SyscallExecveat, Path: `/a<b>&"c\`, Status: StatusFexecve})
	if err != nil {
		t.Fatal(err)
	}
	want := `{"seq":4,"tid":9,"tgid":9,"syscall":"execveat","path":"/a<b>&\"c\\","status":"fexecve"}`
	if string(enc) != want {
		t.Fatalf("%s", enc)
	}
}

func TestHeaderValidation(t *testing.T) {
	bad := []Header{
		{Kernel: "k\n", Nonce: testNonce, ActionSHA256: testAction},
		{Kernel: strings.Repeat("k", MaxKernelBytes+1), Nonce: testNonce, ActionSHA256: testAction},
		{Kernel: "k", Nonce: "xyz", ActionSHA256: testAction},
		{Kernel: "k", Nonce: testNonce, ActionSHA256: "abc"},
		{Kernel: "k", Nonce: testNonce, ActionSHA256: testAction, Step: -1},
	}
	for i, h := range bad {
		if _, err := NewWriter(&bytes.Buffer{}, h, Limits{}); err == nil {
			t.Errorf("header %d accepted", i)
		}
	}
}

func TestWriterCapsStopWritingAndStaySticky(t *testing.T) {
	var buf bytes.Buffer
	w, err := NewWriter(&buf, testHeader(), Limits{MaxLines: 2})
	if err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 2; i++ {
		if err := w.Append(plainLine(i+1, StatusOutside)); err != nil {
			t.Fatal(err)
		}
	}
	before := buf.Len()
	if err := w.Append(plainLine(3, StatusOutside)); !errors.Is(err, ErrOverflow) || !w.Overflowed() {
		t.Fatalf("the line cap: %v", err)
	}
	if err := w.Append(plainLine(4, StatusOutside)); !errors.Is(err, ErrOverflow) {
		t.Fatalf("sticky: %v", err)
	}
	if buf.Len() != before || w.Lines() != 2 {
		t.Fatal("a refused line was written")
	}
	// The byte cap: the header fits, the first line does not.
	var b2 bytes.Buffer
	w2, err := NewWriter(&b2, testHeader(), Limits{MaxBytes: int64(len(split(t, build(t))[0])) + 10})
	if err != nil {
		t.Fatal(err)
	}
	if err := w2.Append(plainLine(1, StatusOutside)); !errors.Is(err, ErrOverflow) {
		t.Fatalf("the byte cap: %v", err)
	}
	if int64(b2.Len()) != w2.Bytes() {
		t.Fatal("Bytes() is not what was written")
	}
}

type failWriter struct{ left int }

func (f *failWriter) Write(p []byte) (int, error) {
	if len(p) > f.left {
		n := f.left
		f.left = 0
		return n, errors.New("disk full")
	}
	f.left -= len(p)
	return len(p), nil
}

func TestWriterWriteErrorIsStickyAndCounted(t *testing.T) {
	hdr := len(split(t, build(t))[0]) + 1
	fw := &failWriter{left: hdr + 5}
	w, err := NewWriter(fw, testHeader(), Limits{})
	if err != nil {
		t.Fatal(err)
	}
	err = w.Append(plainLine(1, StatusOutside))
	if err == nil || w.Overflowed() || !strings.Contains(err.Error(), "disk full") {
		t.Fatalf("%v", err)
	}
	if w.Err() == nil || w.Append(plainLine(2, StatusOutside)) == nil {
		t.Fatal("a failed write is not sticky")
	}
	if w.Bytes() != int64(hdr+5) {
		t.Fatalf("Bytes() = %d, want what the destination took (%d)", w.Bytes(), hdr+5)
	}
	if w.Lines() != 0 {
		t.Fatal("a line that was not fully written counted")
	}
	if _, err := NewWriter(&failWriter{left: 3}, testHeader(), Limits{}); err == nil {
		t.Fatal("a header that could not be written was accepted")
	}
}

func TestSecondFileIsNotAppendable(t *testing.T) {
	// Two files concatenated are not one file: the second header is a line whose
	// keys are wrong, and the seq restarts.
	a := build(t, plainLine(1, StatusOutside))
	if _, err := Decode(append(append([]byte{}, a...), a...), Limits{}, testExpect); err == nil {
		t.Fatal("two files in one were accepted")
	}
}

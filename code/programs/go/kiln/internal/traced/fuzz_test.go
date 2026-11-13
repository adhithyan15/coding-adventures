package traced

import (
	"sort"
	"strings"
	"testing"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// FuzzClassify throws arbitrary event sequences at Classify.  The properties:
//
//   - it never panics and never hangs (symlink loops, "..", deep paths);
//   - every list in the record is sorted and free of duplicates;
//   - every entry is pkg:, path: or list: with a clean repository path;
//   - no reason carries a control character or runs past its cap.
//
// The bytes drive a small interpreter: each byte picks an operation, the
// following bytes pick names from a pool that deliberately includes links,
// "..", the repository root and /proc.
func FuzzClassify(f *testing.F) {
	f.Add([]byte{0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12})
	f.Add([]byte{3, 3, 3, 4, 4, 5, 5, 6, 7, 0, 0, 0})
	f.Add([]byte("symlink loop ../.. /proc/self/status"))
	pool := []string{
		"code/p/a/README", "code/p/a/lnk", "code/p/a/lnk/../x", "code/p/b/b.txt", "code/p/a/nested/new",
		"go.work", ".git/config", "../outside", "/usr/bin/cc", "/proc/self/status", "/proc/42/fd/3",
		"/dev/fd/3", "/tmp/kiln-1/t", "L1", "L2", "L1/x", "a/b/c/d", "", ".", "..", "/", "x\ny",
	}
	f.Fuzz(func(t *testing.T, data []byte) {
		h := newHarness(t)
		h.fs.link(abs("L1"), "L2")
		h.fs.link(abs("L2"), "L1")
		h.fs.link(abs("code/p/a/lnk"), "../b")
		var evs []trace.Event
		pick := func(i int) string { return pool[int(data[i%len(data)])%len(pool)] }
		for i := 0; i+1 < len(data) && len(evs) < 64; i += 2 {
			p := pick(i + 1)
			var e trace.Event
			switch data[i] % 12 {
			case 0:
				e = open(p, "O_RDONLY", abs("code/p/b/b.txt"))
			case 1:
				e = open(p, "O_WRONLY|O_CREAT|O_EXCL", pick(i+2))
			case 2:
				e = stat(p)
			case 3:
				e = fail(stat(p), "ENOENT")
			case 4:
				e = mkdir(p)
			case 5:
				e = symlink(pick(i+2), p)
			case 6:
				e = rename(p, pick(i+2))
			case 7:
				e = link(p, pick(i+2))
			case 8:
				e = write("unlink", p)
			case 9:
				e = getdents(abs(p))
			case 10:
				e = execve(p, "x")
			default:
				e = inet("connect", "127.0.0.1", int(data[i])%100)
			}
			if data[i]&0x80 != 0 {
				e = fail(e, "EEXIST")
			}
			evs = append(evs, e)
		}
		if len(evs) == 0 {
			return
		}
		rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{step(evs...)}, Recipe: []string{"step"}})
		if err != nil {
			t.Fatal(err)
		}
		for _, l := range [][]string{rec.Entries, rec.Writes, rec.TrackedWrites, rec.UntrackedObservations, rec.Traversed, rec.Reasons} {
			if !sort.StringsAreSorted(l) {
				t.Fatalf("unsorted %q", l)
			}
			for i := 1; i < len(l); i++ {
				if l[i-1] == l[i] {
					t.Fatalf("duplicate %q", l[i])
				}
			}
		}
		for _, e := range rec.Entries {
			kind, name, ok := strings.Cut(e, ":")
			if !ok || (kind != "pkg" && kind != "path" && kind != "list") || !utf8.ValidString(name) ||
				strings.HasPrefix(name, "/") || strings.Contains(name, "..") || strings.HasSuffix(name, "/") {
				t.Fatalf("bad entry %q", e)
			}
		}
		for _, r := range rec.Reasons {
			if strings.ContainsAny(r, "\n\r\x00") || len(r) > 300 {
				t.Fatalf("bad reason %q", r)
			}
		}
	})
}

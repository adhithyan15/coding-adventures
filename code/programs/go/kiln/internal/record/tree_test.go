package record

import (
	"bytes"
	"crypto/sha1"
	"crypto/sha256"
	"fmt"
	"os"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// blobID is a Git blob id (SHA-1) of content.
func blobID(content string) string {
	h := sha1.New()
	fmt.Fprintf(h, "blob %d\x00%s", len(content), content)
	return fmt.Sprintf("%x", h.Sum(nil))
}

func blobID256(content string) string {
	h := sha256.New()
	fmt.Fprintf(h, "blob %d\x00%s", len(content), content)
	return fmt.Sprintf("%x", h.Sum(nil))
}

func fixtureTree() (map[string]entry, func(string) (string, error)) {
	links := map[string]string{"code/l": "../x y", "code/m": "a"}
	tree := map[string]entry{
		"code/a.go":         {modeFile, blobID("a")},
		"code/run.sh":       {modeExec, blobID("#!/bin/sh\n")},
		"code/l":            {modeSymlink, blobID(links["code/l"])},
		"code/m":            {modeSymlink, blobID(links["code/m"])},
		"code/sp ace/é.txt": {modeFile, blobID("é")},
		"code/pct%/x":       {modeFile, blobID("x")},
		"top":               {modeFile, blobID("t")},
	}
	return tree, func(p string) (string, error) {
		if t, ok := links[p]; ok {
			return t, nil
		}
		return "", os.ErrNotExist
	}
}

func TestTreeIsByteExactDeterministicAndRoundTrips(t *testing.T) {
	tree, link := fixtureTree()
	a, err := EncodeTree(tree, link)
	if err != nil {
		t.Fatal(err)
	}
	b, _ := EncodeTree(tree, link)
	if string(a) != string(b) {
		t.Fatal("not deterministic")
	}
	want := "kiln-record-tree/1\n" +
		"100644 " + blobID("a") + " code/a.go\n" +
		"120000 " + blobID("../x y") + " code/l ../x%20y\n" +
		"120000 " + blobID("a") + " code/m a\n" +
		"100644 " + blobID("x") + " code/pct%25/x\n" +
		"100755 " + blobID("#!/bin/sh\n") + " code/run.sh\n" +
		"100644 " + blobID("é") + " code/sp%20ace/%C3%A9.txt\n" +
		"100644 " + blobID("t") + " top\n"
	if string(a) != want {
		t.Fatalf("got\n%s\nwant\n%s", a, want)
	}
	d, err := DecodeTree(a)
	if err != nil {
		t.Fatal(err)
	}
	if d.Len() != len(tree) {
		t.Fatalf("%d entries", d.Len())
	}
	for _, p := range d.Paths() {
		mode, id, text, ok := d.Entry(p)
		if !ok || mode != tree[p].mode || id != tree[p].id {
			t.Errorf("%s: %s %s", p, mode, id)
		}
		if mode == modeSymlink {
			if want, _ := link(p); text != want {
				t.Errorf("link %s: %q", p, text)
			}
		}
	}
	// Stat: files, links with their text, directories, absent.
	for path, want := range map[string]traced.Node{
		"":                  {Kind: traced.KindDir},
		"code":              {Kind: traced.KindDir},
		"code/sp ace":       {Kind: traced.KindDir},
		"code/a.go":         {Kind: traced.KindFile},
		"code/run.sh":       {Kind: traced.KindFile},
		"code/l":            {Kind: traced.KindLink, Target: "../x y"},
		"code/sp ace/é.txt": {Kind: traced.KindFile},
		"code/nope":         {},
		"cod":               {},
	} {
		if got := d.Stat(path); !reflect.DeepEqual(got, want) {
			t.Errorf("Stat(%q) = %+v, want %+v", path, got, want)
		}
	}
	// The decode is the encoder's inverse: re-encoding what was decoded gives the
	// same bytes.
	again := map[string]entry{}
	for _, p := range d.Paths() {
		m, id, _, _ := d.Entry(p)
		again[p] = entry{m, id}
	}
	c, err := EncodeTree(again, func(p string) (string, error) { _, _, t, _ := d.Entry(p); return t, nil })
	if err != nil || string(c) != string(a) {
		t.Fatalf("re-encoded differently: %v", err)
	}
}

func TestTreeAcceptsASHA256Repository(t *testing.T) {
	tree := map[string]entry{"a": {modeFile, blobID256("a")}, "l": {modeSymlink, blobID256("a")}}
	data, err := EncodeTree(tree, func(string) (string, error) { return "a", nil })
	if err != nil {
		t.Fatal(err)
	}
	if _, err := DecodeTree(data); err != nil {
		t.Fatal(err)
	}
}

func TestEncodeTreeRefusals(t *testing.T) {
	good := blobID("a")
	ok := func(p string) (string, error) { return "a", nil }
	for name, c := range map[string]struct {
		tree map[string]entry
		link func(string) (string, error)
	}{
		"a gitlink":              {map[string]entry{"s": {modeGitlink, good}}, ok},
		"an odd mode":            {map[string]entry{"s": {"100664", good}}, ok},
		"a short id":             {map[string]entry{"s": {modeFile, "abc"}}, ok},
		"an upper-case id":       {map[string]entry{"s": {modeFile, strings.ToUpper(good)}}, ok},
		"mixed id lengths":       {map[string]entry{"a": {modeFile, good}, "b": {modeFile, blobID256("a")}}, ok},
		"a .git path":            {map[string]entry{".git/x": {modeFile, good}}, ok},
		"a dot-dot path":         {map[string]entry{"a/../b": {modeFile, good}}, ok},
		"a link nobody can read": {map[string]entry{"l": {modeSymlink, good}}, func(string) (string, error) { return "", os.ErrNotExist }},
		"a link with no reader":  {map[string]entry{"l": {modeSymlink, good}}, nil},
		"a link not its blob":    {map[string]entry{"l": {modeSymlink, good}}, func(string) (string, error) { return "b", nil }},
		"an empty link text":     {map[string]entry{"l": {modeSymlink, good}}, func(string) (string, error) { return "", nil }},
		"a link text with NUL":   {map[string]entry{"l": {modeSymlink, good}}, func(string) (string, error) { return "a\x00", nil }},
	} {
		if _, err := EncodeTree(c.tree, c.link); err == nil {
			t.Errorf("%s: encoded", name)
		}
	}
}

func TestDecodeTreeIsStrict(t *testing.T) {
	id, id2 := blobID("a"), blobID("b")
	hdr := TreeSchema + "\n"
	line := func(mode, id, p string) string { return mode + " " + id + " " + p + "\n" }
	if d, err := DecodeTree([]byte(hdr)); err != nil || d.Len() != 0 {
		t.Fatalf("an empty tree: %v", err)
	}
	for name, in := range map[string]string{
		"no header":            line("100644", id, "a"),
		"another schema":       "kiln-record-tree/2\n" + line("100644", id, "a"),
		"no final newline":     hdr + strings.TrimSuffix(line("100644", id, "a"), "\n"),
		"unsorted":             hdr + line("100644", id, "b") + line("100644", id, "a"),
		"duplicate":            hdr + line("100644", id, "a") + line("100644", id2, "a"),
		"a file and its dir":   hdr + line("100644", id, "a") + line("100644", id, "a/b"),
		"bad mode":             hdr + line("100664", id, "a"),
		"gitlink":              hdr + line("160000", id, "a"),
		"short id":             hdr + line("100644", "abc", "a"),
		"mixed id lengths":     hdr + line("100644", id, "a") + line("100644", blobID256("a"), "b"),
		"link without text":    hdr + line("120000", id, "a"),
		"file with link text":  hdr + "100644 " + id + " a b\n",
		"two spaces":           hdr + "100644  " + id + " a\n",
		"a raw space in path":  hdr + "100644 " + id + " a b c\n",
		"a raw high byte":      hdr + line("100644", id, "\xc3\xa9"),
		"lower-case escape":    hdr + line("100644", id, "a%2fb"),
		"needless escape":      hdr + line("100644", id, "%61"),
		"truncated escape":     hdr + line("100644", id, "a%2"),
		"non-hex escape":       hdr + line("100644", id, "a%ZZ"),
		"a leading slash":      hdr + line("100644", id, "%2Fa"),
		"a .git component":     hdr + line("100644", id, ".git/config"),
		"an empty path":        hdr + "100644 " + id + " \n",
		"an empty line":        hdr + "\n",
		"an escaped NUL path":  hdr + line("100644", id, "a%00b"),
		"link text escape bad": hdr + "120000 " + id + " l %zz\n",
		"empty link text":      hdr + "120000 " + id + " l \n",
		"a very long line":     hdr + line("100644", id, strings.Repeat("a", maxTreeLine)),
	} {
		if _, err := DecodeTree([]byte(in)); err == nil {
			t.Errorf("%s: decoded", name)
		}
	}
}

// The escape is canonical: every byte string has one spelling.
func TestTreeTextEscapeRoundTripsEveryByte(t *testing.T) {
	var all []byte
	for i := 0; i < 256; i++ {
		all = append(all, byte(i))
	}
	var b bytes.Buffer
	escapeTreeText(&b, string(all))
	got, err := unescapeTreeText(b.Bytes())
	if err != nil || got != string(all) {
		t.Fatalf("%v", err)
	}
	if strings.ContainsAny(b.String(), " \n\x00") {
		t.Fatal("an escaped field holds a separator")
	}
}

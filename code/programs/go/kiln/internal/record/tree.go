package record

// The sealed tracked tree: `pre/tree` (KILN04 section 4.8, decision P13)
// =====================================================================
//
// `traced.Classify` asks two questions of the tracked tree: "is this path
// tracked, and is it a file, a link (and to what), or a directory?" (Tree.Stat)
// and "what are the first bytes of this tracked file?" (Tree.Head). Until P13
// the answer came from a `source.Source`, which means from Git and from the
// disk, at the time the classifier ran. Classification now happens inside
// `kiln record step`, at the end of each action, with no Git: so what Git said
// about the commit is written down once, by `prepare`, into a file that SEALED
// covers, and `step` reads it back.
//
//	kiln-record-tree/1
//	100644 <blob id> <path>
//	100755 <blob id> <path>
//	120000 <blob id> <path> <link text>
//
// One line per tracked path of the verified commit (`git ls-tree -r`, which
// VerifyTree has already proven equals the disk), sorted bytewise by path, each
// path once. A path or a link text is written with every byte that is not a
// visible ASCII character, and the percent sign, as %XX in UPPER-case hex; that
// makes a field free of spaces and newlines, so a line is split on single
// spaces, and makes the encoding canonical (there is exactly one way to write a
// given byte string), so a file that decodes can be re-encoded to the same
// bytes and a strict decoder can refuse everything else.
//
//	code/packages/go/my pkg/a.go   ->   code/packages/go/my%20pkg/a.go
//	a link text "../x y"           ->   ../x%20y
//
// The real tree has about 394 thousand paths, so the format is line-oriented
// and cheap to read: about 40 MB, hashed in a few tens of milliseconds and
// decoded in a few hundred.
//
// Head. Tree.Head must return the bytes the KEY digests, i.e. the committed
// ones, but the only place to read them without Git is the disk, which a BUILD
// may have changed. So the disk's bytes are used only when they hash, as a Git
// blob, to the blob id this file seals; anything else is an error, and the
// classifier then reads the file through the live disk view and treats it as an
// untracked exec read (which it checks for later changes).

import (
	"bytes"
	"fmt"
	"io"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// TreeSchema is the first line of pre/tree.
const TreeSchema = "kiln-record-tree/1"

// Limits: generous for a real tree, there so a corrupt file cannot exhaust
// memory.
const (
	maxTreeEntries = 1 << 22
	maxTreeLine    = 16 << 10
)

// treeNode is one tracked path.
type treeNode struct {
	mode string
	id   string
	link string // the link text, for mode 120000
}

// TreeFile is a decoded pre/tree.
type TreeFile struct {
	nodes map[string]treeNode
	dirs  map[string]bool
	idLen int
}

// Len is the number of tracked paths.
func (t *TreeFile) Len() int { return len(t.nodes) }

// Stat implements traced.Tree: a file, a link with its text, a directory (a
// path with a tracked path beneath it), or absent.
func (t *TreeFile) Stat(rel string) traced.Node {
	if rel == "" {
		return traced.Node{Kind: traced.KindDir}
	}
	if n, ok := t.nodes[rel]; ok {
		if n.mode == modeSymlink {
			return traced.Node{Kind: traced.KindLink, Target: n.link}
		}
		return traced.Node{Kind: traced.KindFile}
	}
	if t.dirs[rel] {
		return traced.Node{Kind: traced.KindDir}
	}
	return traced.Node{}
}

// Entry returns a tracked path's mode, blob id and link text.
func (t *TreeFile) Entry(rel string) (mode, id, link string, ok bool) {
	n, ok := t.nodes[rel]
	return n.mode, n.id, n.link, ok
}

// Paths lists the tracked paths, sorted.
func (t *TreeFile) Paths() []string {
	out := make([]string, 0, len(t.nodes))
	for p := range t.nodes {
		out = append(out, p)
	}
	sort.Strings(out)
	return out
}

// escapeTreeText writes s with every byte that is not a visible ASCII
// character, and the percent sign, as %XX.
func escapeTreeText(b *bytes.Buffer, s string) {
	const hexUpper = "0123456789ABCDEF"
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c <= 0x20 || c >= 0x7f || c == '%' {
			b.WriteByte('%')
			b.WriteByte(hexUpper[c>>4])
			b.WriteByte(hexUpper[c&15])
			continue
		}
		b.WriteByte(c)
	}
}

// unescapeTreeText is the strict inverse: only %XX with upper-case digits, and
// only for a byte that escapeTreeText would have escaped.
func unescapeTreeText(s []byte) (string, error) {
	if bytes.IndexByte(s, '%') < 0 {
		for _, c := range s {
			if c <= 0x20 || c >= 0x7f {
				return "", fmt.Errorf("an unescaped byte 0x%02X", c)
			}
		}
		return string(s), nil
	}
	out := make([]byte, 0, len(s))
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c != '%' {
			if c <= 0x20 || c >= 0x7f {
				return "", fmt.Errorf("an unescaped byte 0x%02X", c)
			}
			out = append(out, c)
			continue
		}
		if i+2 >= len(s) {
			return "", fmt.Errorf("a truncated %% escape")
		}
		hi, ok1 := upperHex(s[i+1])
		lo, ok2 := upperHex(s[i+2])
		if !ok1 || !ok2 {
			return "", fmt.Errorf("a malformed %% escape")
		}
		v := hi<<4 | lo
		if v > 0x20 && v < 0x7f && v != '%' {
			return "", fmt.Errorf("a needless %% escape of 0x%02X", v)
		}
		out = append(out, v)
		i += 2
	}
	return string(out), nil
}

func upperHex(c byte) (byte, bool) {
	switch {
	case c >= '0' && c <= '9':
		return c - '0', true
	case c >= 'A' && c <= 'F':
		return c - 'A' + 10, true
	}
	return 0, false
}

// EncodeTree renders a pre/tree for the tracked entries (path -> mode and id),
// sorted by path. link supplies the text of a mode-120000 entry; it is checked
// against the entry's blob id (a link text is the blob's whole content), so a
// tree whose links do not hash to what Git says is never written. Gitlinks are
// refused (VerifyTree already did).
func EncodeTree(entries map[string]entry, link func(path string) (string, error)) ([]byte, error) {
	paths := make([]string, 0, len(entries))
	for p := range entries {
		paths = append(paths, p)
	}
	sort.Strings(paths)
	if len(paths) > maxTreeEntries {
		return nil, fmt.Errorf("tree: %d tracked paths exceed %d", len(paths), maxTreeEntries)
	}
	var b bytes.Buffer
	b.Grow(len(paths)*96 + 32)
	b.WriteString(TreeSchema)
	b.WriteByte('\n')
	idLen := 0
	for _, p := range paths {
		e := entries[p]
		if idLen == 0 {
			idLen = len(e.id)
		}
		if len(e.id) != idLen || !isHex(e.id) {
			return nil, fmt.Errorf("tree: %q: object id %q is not %d lowercase hex digits", printable(p), printable(e.id), idLen)
		}
		if _, err := newHasher(e.id); err != nil {
			return nil, fmt.Errorf("tree: %q: %v", printable(p), err)
		}
		if why := checkTreePath(p); why != "" {
			return nil, fmt.Errorf("tree: %q: %s", printable(p), why)
		}
		b.WriteString(e.mode)
		b.WriteByte(' ')
		b.WriteString(e.id)
		b.WriteByte(' ')
		escapeTreeText(&b, p)
		switch e.mode {
		case modeFile, modeExec:
		case modeSymlink:
			if link == nil {
				return nil, fmt.Errorf("tree: %q is a symlink and no link reader was given", printable(p))
			}
			text, err := link(p)
			if err != nil {
				return nil, fmt.Errorf("tree: link %q: %v", printable(p), err)
			}
			if err := checkLinkText(text); err != nil {
				return nil, fmt.Errorf("tree: link %q: %v", printable(p), err)
			}
			h, _ := newHasher(e.id)
			writeBlobHeader(h, int64(len(text)))
			h.Write([]byte(text))
			if got := fmt.Sprintf("%x", h.Sum(nil)); got != e.id {
				return nil, fmt.Errorf("tree: link %q: its text does not hash to the blob id %s", printable(p), short(e.id))
			}
			b.WriteByte(' ')
			escapeTreeText(&b, text)
		default:
			return nil, fmt.Errorf("tree: %q has mode %q, which a tree file does not hold", printable(p), printable(e.mode))
		}
		b.WriteByte('\n')
	}
	return b.Bytes(), nil
}

func checkLinkText(s string) error {
	if s == "" || strings.IndexByte(s, 0) >= 0 {
		return fmt.Errorf("a link text must be non-empty and hold no NUL")
	}
	return nil
}

// DecodeTree reads a pre/tree strictly: the exact header, one line per entry in
// strictly ascending path order, canonical escapes, a link text only on a
// symlink, one object-id length throughout, no path that is both a file and a
// directory, and a final newline.
func DecodeTree(data []byte) (*TreeFile, error) {
	hdr := TreeSchema + "\n"
	if !bytes.HasPrefix(data, []byte(hdr)) {
		return nil, fmt.Errorf("tree: the header is not %q", TreeSchema)
	}
	data = data[len(hdr):]
	if len(data) > 0 && data[len(data)-1] != '\n' {
		return nil, fmt.Errorf("tree: no newline at the end")
	}
	t := &TreeFile{nodes: make(map[string]treeNode, bytes.Count(data, []byte{'\n'})), dirs: map[string]bool{}}
	prev := ""
	for n := 1; len(data) > 0; n++ {
		i := bytes.IndexByte(data, '\n')
		line := data[:i]
		data = data[i+1:]
		if len(line) > maxTreeLine {
			return nil, fmt.Errorf("tree line %d is longer than %d bytes", n, maxTreeLine)
		}
		if n > maxTreeEntries {
			return nil, fmt.Errorf("tree: more than %d entries", maxTreeEntries)
		}
		node, p, err := parseTreeLine(line)
		if err != nil {
			return nil, fmt.Errorf("tree line %d: %v", n, err)
		}
		if t.idLen == 0 {
			t.idLen = len(node.id)
		} else if len(node.id) != t.idLen {
			return nil, fmt.Errorf("tree line %d: an object id of %d digits among ids of %d", n, len(node.id), t.idLen)
		}
		if n > 1 && p <= prev {
			return nil, fmt.Errorf("tree line %d: %q is not after %q (sorted, each path once)", n, printable(p), printable(prev))
		}
		prev = p
		t.nodes[p] = node
		for d := p; ; {
			j := strings.LastIndexByte(d, '/')
			if j < 0 {
				break
			}
			d = d[:j]
			if t.dirs[d] {
				break
			}
			t.dirs[d] = true
		}
	}
	for d := range t.dirs {
		if _, isFile := t.nodes[d]; isFile {
			return nil, fmt.Errorf("tree: %q is both a tracked file and a directory", printable(d))
		}
	}
	return t, nil
}

// parseTreeLine splits "mode id path[ link]".
func parseTreeLine(line []byte) (treeNode, string, error) {
	f := bytes.Split(line, []byte{' '})
	if len(f) != 3 && len(f) != 4 {
		return treeNode{}, "", fmt.Errorf("not \"<mode> <id> <path>[ <link text>]\"")
	}
	var n treeNode
	switch mode := string(f[0]); mode {
	case modeFile, modeExec:
		if len(f) != 3 {
			return treeNode{}, "", fmt.Errorf("a regular file has no link text")
		}
		n.mode = mode
	case modeSymlink:
		if len(f) != 4 {
			return treeNode{}, "", fmt.Errorf("a symlink needs its link text")
		}
		n.mode = mode
	default:
		return treeNode{}, "", fmt.Errorf("mode %q is not 100644, 100755 or 120000", printable(mode))
	}
	n.id = string(f[1])
	if _, err := newHasher(n.id); err != nil || !isHex(n.id) {
		return treeNode{}, "", fmt.Errorf("the object id is not 40 or 64 lowercase hex digits")
	}
	p, err := unescapeTreeText(f[2])
	if err != nil {
		return treeNode{}, "", fmt.Errorf("path: %v", err)
	}
	if why := checkTreePath(p); why != "" {
		return treeNode{}, "", fmt.Errorf("path %q: %s", printable(p), why)
	}
	if len(f) == 4 {
		if n.link, err = unescapeTreeText(f[3]); err != nil {
			return treeNode{}, "", fmt.Errorf("link text: %v", err)
		}
		if err := checkLinkText(n.link); err != nil {
			return treeNode{}, "", err
		}
	}
	return n, p, nil
}

// DiskTree is the traced.Tree the step's epilogue hands the classifier: the
// sealed tree for Stat, and the live disk, verified against the sealed blob id,
// for Head.
type DiskTree struct {
	*TreeFile
	// Root is the checkout.
	Root string
	// heads caches the verified head of each tracked file already read: a
	// script run in a loop is read once.
	heads map[string][]byte
}

// NewDiskTree wraps a decoded tree and the checkout it describes.
func NewDiskTree(t *TreeFile, root string) *DiskTree {
	return &DiskTree{TreeFile: t, Root: root, heads: map[string][]byte{}}
}

// maxVerifiedFile bounds the size of a tracked file Head will hash. A larger
// one is an error, which the classifier handles as an untracked exec read.
const maxVerifiedFile = 256 << 20

// Head implements traced.Tree: up to n leading bytes of the tracked regular
// file, but only when the disk's whole file hashes, as a Git blob, to the id
// the sealed tree holds for it.
func (d *DiskTree) Head(rel string, n int) ([]byte, error) {
	node, ok := d.nodes[rel]
	if !ok || node.mode == modeSymlink {
		return nil, fmt.Errorf("record: not a tracked regular file")
	}
	if b, ok := d.heads[rel]; ok {
		return b[:min(n, len(b))], nil
	}
	b, err := readVerifiedHead(d.Root+"/"+rel, node.id, maxVerifiedFile, headCacheBytes)
	if err != nil {
		return nil, err
	}
	d.heads[rel] = b
	return b[:min(n, len(b))], nil
}

// headCacheBytes is how many leading bytes of a verified file are kept: what
// the classifier's exec rules ever ask for (traced's headLimit).
const headCacheBytes = 64 << 10

// captureWriter keeps the first limit bytes written to it.
type captureWriter struct {
	buf   []byte
	limit int
}

func (c *captureWriter) Write(p []byte) (int, error) {
	if room := c.limit - len(c.buf); room > 0 {
		c.buf = append(c.buf, p[:min(room, len(p))]...)
	}
	return len(p), nil
}

var _ io.Writer = (*captureWriter)(nil)

package legacy

// Escape, indirection and unscanned rules (KILN03 section 6).
//
// A legacy BUILD can reach outside its closure in ways a key cannot see.  The
// closure and reads (KILN03 3.2, 3.3) are what the key covers; anything a
// package's files can point at beyond them is a hole.  Three cheap,
// conservative rules look for such pointers in the package's OWN files:
//
//	../ resolution   a path token with a `..` segment, resolved against the
//	                 directory of the file it appears in, must land inside the
//	                 package, a closure member or a read path       escape:<path>
//	indirection      markers that compute a location at run time
//	                 (`dirname`, `$PWD`, `git rev-parse`, ...), a bare
//	                 `cd ..`, or an absolute path                   indirect:<file>
//	unscanned        a file the scanner cannot read as text:
//	                 text over the size cap, or an executable that is not
//	                 text                                           unscanned:<file>
//
// They are heuristics, and every doubt is resolved toward a hit (a false
// positive costs cache hit rate; a false negative would be a false cache hit).
//
// The per-file part (what is in this file?) depends only on the file, so it is
// computed once, in the same pass that hashes the file (hashAndScan).  The
// per-package part (is that target inside THIS package's closure?) is done in
// scanReasons.

import (
	"io"
	"path"
	"regexp"
	"sort"
	"strings"
	"unicode/utf8"

	"hash"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

const (
	// maxScanBytes is the size cap: a text file larger than this is not
	// scanned and makes its package uncacheable (KILN03 6.3).
	maxScanBytes = 256 << 10
	// headBytes is the window used to decide text versus binary.
	headBytes = 8 << 10
	// maxScanReasons bounds how many reasons of one kind a package records, so
	// a package with thousands of offending files cannot bloat the plan.  The
	// kept ones are the first in sorted order; the class is unaffected.
	maxScanReasons = 16
)

// scanState says what became of one file.
type scanState uint8

const (
	// scanSkipped: not text and not executable (an image, an archive, ...).
	// Such a file cannot be interpreted as a script by the BUILD runner, so it
	// is hashed but not scanned.  (Everything is still in the key.)
	scanSkipped scanState = iota
	// scanText: text within the cap; refs, dynamic and indirect are valid.
	scanText
	// scanUnscanned: text over the cap, or an executable that is not text.
	scanUnscanned
)

// fileFacts is everything the per-file scan learns.
type fileFacts struct {
	state    scanState
	indirect bool     // a marker or an absolute path (KILN03 6.2)
	refs     []string // sorted, unique, repo-relative cleaned `..` targets; may start with "../" (leaves the repo)
	dynamic  []string // sorted, unique `..` tokens that cannot be resolved statically ($VAR, globs, ...)
}

// isText is the text definition of KILN03 11: the head of the file (its first
// 8 KiB, or all of it when shorter) contains no NUL byte and is valid UTF-8.
// A multi-byte character cut in half by the end of the window is not an error.
func isText(head []byte) bool {
	for _, b := range head {
		if b == 0 {
			return false
		}
	}
	if len(head) == headBytes {
		// Drop an incomplete final character: find the last start byte in the
		// final UTFMax-1 bytes and cut there if that character is unfinished.
		for i := 1; i < utf8.UTFMax && i <= len(head); i++ {
			if utf8.RuneStart(head[len(head)-i]) {
				if !utf8.FullRune(head[len(head)-i:]) {
					head = head[:len(head)-i]
				}
				break
			}
		}
	}
	return utf8.Valid(head)
}

// hashAndScan reads rc once: every byte goes to h (the content digest), and the
// first maxScanBytes are kept so they can be scanned.  The digest is exactly
// what io.Copy(h, rc) would produce.
//
//	head <= 8 KiB   decides text / binary
//	binary          executable -> unscanned, otherwise skipped
//	text, <= cap    scanned
//	text, > cap     unscanned (the remainder is only hashed)
func hashAndScan(rc io.Reader, h hash.Hash, exec bool, dir string) (fileFacts, error) {
	head, err := io.ReadAll(io.LimitReader(rc, headBytes))
	if err != nil {
		return fileFacts{}, err
	}
	h.Write(head)
	if !isText(head) {
		if _, err := io.Copy(h, rc); err != nil {
			return fileFacts{}, err
		}
		if exec {
			return fileFacts{state: scanUnscanned}, nil
		}
		return fileFacts{state: scanSkipped}, nil
	}
	rest, err := io.ReadAll(io.LimitReader(rc, int64(maxScanBytes+1-len(head))))
	if err != nil {
		return fileFacts{}, err
	}
	h.Write(rest)
	if len(head)+len(rest) > maxScanBytes {
		if _, err := io.Copy(h, rc); err != nil {
			return fileFacts{}, err
		}
		return fileFacts{state: scanUnscanned}, nil
	}
	f := scanContent(dir, string(head)+string(rest))
	f.state = scanText
	return f, nil
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

// delims are the bytes that end a token: whitespace, quotes (including the
// backtick), `=`, `(`, `)`, `,`, `;`, `:`, and the shell operators `<`, `>`,
// `|`, `&`.  So `--flag=../x`, `"../x"`, `cp a ../x;`, `PATH=a:../b` and
// `$(cat ../x)` all yield a token that starts at the path.  Everything else
// (`$`, `{`, `/`, `\`, `.`, `-`, ...) belongs to the token.
var delims = func() (t [256]bool) {
	for _, c := range " \t\r\n\f\v\"'`=(),;:<>|&" {
		t[c] = true
	}
	return
}()

// forEachToken calls fn with every maximal run of non-delimiter bytes.
func forEachToken(s string, fn func(tok string)) {
	start := -1
	for i := 0; i < len(s); i++ {
		if delims[s[i]] {
			if start >= 0 {
				fn(s[start:i])
				start = -1
			}
		} else if start < 0 {
			start = i
		}
	}
	if start >= 0 {
		fn(s[start:])
	}
}

// urlRE matches a URL: a scheme, "://" and the run of characters up to the
// next whitespace, quote, bracket or operator.  A URL such as
// `https://x/../y` is not a file-system path, so it is removed before
// tokenising (KILN03 11).  `file://` URLs ARE paths and are kept.
var urlRE = regexp.MustCompile("(?i)[a-z][a-z0-9+.-]*://[^\\s\"'`<>()|;,]*")

func stripURLs(s string) string {
	if !strings.Contains(s, "://") {
		return s
	}
	return urlRE.ReplaceAllStringFunc(s, func(m string) string {
		if len(m) >= 5 && strings.EqualFold(m[:5], "file:") {
			return m
		}
		return " "
	})
}

// dynamicBytes make a `..` token impossible to resolve statically: shell
// variables, command substitution remnants, brace/glob patterns, `~` and
// Windows `%VAR%`.  Such a token is an escape (fail closed).
const dynamicBytes = "$*?[]{}~%"

// dotdot classifies one token that contains "..".  It reports the repo
// relative target of a path token with a `..` segment, or the token itself when
// it cannot be resolved.
//
//	"../x"            -> dir/../x cleaned
//	`..\x`            -> same (a backslash counts as a separator)
//	"-I../inc"        -> a glued flag prefix is dropped: "../inc"
//	"a/.."            -> the trailing ".." is a segment too
//	"$ROOT/../x"      -> dynamic
//	"/a/../b"         -> neither: absolute paths are the indirection rule's
//	"wait..", "a..b"  -> neither: ".." must be a whole segment next to a "/"
func dotdot(dir, tok string) (target string, dynamic bool, ok bool) {
	t := strings.ReplaceAll(tok, `\`, "/")
	if strings.HasSuffix(t, "/..") {
		t += "/"
	}
	i := strings.Index(t, "../")
	if i < 0 {
		return "", false, false
	}
	if i > 0 && !strings.Contains(t[:i], "/") {
		t = t[i:] // glued prefix such as -I or @
	}
	if strings.HasPrefix(t, "/") {
		return "", false, false
	}
	if strings.ContainsAny(t, dynamicBytes) {
		return t, true, true
	}
	return path.Join(dir, t), false, true
}

// ---------------------------------------------------------------------------
// Indirection
// ---------------------------------------------------------------------------

// Substring markers (case sensitive).  Plain substring matching is the
// conservative choice: `dirname` also fires on `__dirname`, `os.path.dirname`
// and `mydirnamex`, which are exactly the shapes that matter, at the price of
// the odd false positive.
var markers = []string{
	"dirname", "realpath", "readlink",
	"$(pwd", "`pwd`", "$PWD", "${PWD", "$OLDPWD", "${OLDPWD",
	"$0", "${0", "$BASH_SOURCE", "${BASH_SOURCE",
	"$HOME", "${HOME", "~/",
	// Windows and PowerShell analogues of $0 and $PWD.
	"%~dp0", "%CD%", "%cd%", "$PSScriptRoot",
}

var (
	gitRevParseRE = regexp.MustCompile(`\bgit\s+rev-parse`)
	gitLsFilesRE  = regexp.MustCompile(`\bgit\s+ls-files`)
	// A bare `cd ..`: the word cd, whitespace, optional quote, "..", any
	// number of slashes, optional quote, then the end of the argument.  `cd
	// ../x` is not bare: it is a `../` token and is judged by where it lands.
	cdUpRE = regexp.MustCompile(`(?m)(?:^|[^A-Za-z0-9_.-])cd[ \t]+["']?\.\.` + `/*["']?(?:[\s;&|)]|$)`)
)

func hasMarker(s string) bool {
	for _, m := range markers {
		if strings.Contains(s, m) {
			return true
		}
	}
	// The regexes are guarded by a cheap substring test: most files contain
	// none of these words, and Go's regexp engine is not fast.
	if strings.Contains(s, "rev-parse") && gitRevParseRE.MatchString(s) {
		return true
	}
	if strings.Contains(s, "ls-files") && gitLsFilesRE.MatchString(s) {
		return true
	}
	return strings.Contains(s, "cd") && strings.Contains(s, "..") && cdUpRE.MatchString(s)
}

// allowedAbsolute are the absolute paths a BUILD may name without it being
// indirection (KILN03 6.2).
var allowedAbsolute = map[string]bool{"/dev/null": true, "/usr/bin/env": true, "/bin/sh": true}

// absoluteOutside reports whether a token is an absolute path outside the
// allowlist.  Rules, all biased toward reporting:
//
//   - a leading "#!" (a shebang) is ignored, so `#!/bin/bash` is the path /bin/bash;
//   - a compiler-style flag glued to a path (`-I/usr/include`) counts as that path;
//   - the token must start with "/" followed by a letter, digit or one of
//     . _ ~ $ @ % + -   (so a lone "/", "/*" and "/^regex/" are not paths);
//   - a token starting with exactly two slashes is a comment marker
//     (`//go:build`) and is ignored, unless it contains ".." or has three or
//     more slashes (`///etc/x`, the tail of a file:// URL).
func absoluteOutside(tok string) bool {
	t := strings.TrimPrefix(tok, "#!")
	if len(t) >= 3 && t[0] == '-' && (t[1]|0x20 >= 'a' && t[1]|0x20 <= 'z') && t[2] == '/' {
		t = t[2:]
	}
	if len(t) < 2 || t[0] != '/' || allowedAbsolute[t] {
		return false
	}
	switch c := t[1]; {
	case c == '/':
		return (len(t) >= 3 && t[2] == '/') || strings.Contains(t, "..")
	case c == '*':
		return false
	case c >= '0' && c <= '9', c|0x20 >= 'a' && c|0x20 <= 'z':
		return true
	default:
		return strings.IndexByte("._~$@%+-", c) >= 0
	}
}

// scanContent is the per-file scan of a text file in directory dir.
func scanContent(dir, s string) fileFacts {
	var f fileFacts
	f.indirect = hasMarker(s)
	refs := map[string]bool{}
	dyn := map[string]bool{}
	forEachToken(stripURLs(s), func(tok string) {
		if !f.indirect && absoluteOutside(tok) {
			f.indirect = true
		}
		if !strings.Contains(tok, "..") {
			return
		}
		if target, dynamic, ok := dotdot(dir, tok); ok {
			if dynamic {
				dyn[target] = true
			} else {
				refs[target] = true
			}
		}
	})
	f.refs = sortedKeys(refs)
	f.dynamic = sortedKeys(dyn)
	return f
}

func sortedKeys(m map[string]bool) []string {
	if len(m) == 0 {
		return nil
	}
	out := make([]string, 0, len(m))
	for k := range m {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}

// ---------------------------------------------------------------------------
// Per-package decisions
// ---------------------------------------------------------------------------

// allowSet is the region a package may point into without escaping: its own
// directory, every closure member's directory, and every read path.
type allowSet struct {
	dirs  map[string]bool // a target at or below any of these is inside
	globs []string        // read globs
}

func (pl *planner) newAllowSet(members []int, items []readItem) *allowSet {
	a := &allowSet{dirs: map[string]bool{}}
	for _, q := range members {
		a.dirs[pl.g.Packages[q].RelPath] = true
	}
	for _, it := range items {
		if it.glob {
			a.globs = append(a.globs, it.path)
		} else {
			a.dirs[it.path] = true
		}
	}
	return a
}

// ok reports whether a repo-relative cleaned target is inside the region.  A
// target that climbs out of the repository never is.
func (a *allowSet) ok(target string) bool {
	if target == ".." || strings.HasPrefix(target, "../") {
		return false
	}
	for d := target; ; {
		if a.dirs[d] {
			return true
		}
		cut := strings.LastIndexByte(d, '/')
		if cut < 0 {
			break
		}
		d = d[:cut]
	}
	for _, g := range a.globs {
		if source.Match(g, target) {
			return true
		}
	}
	return false
}

// printable strips control characters so a file name from the tree cannot
// forge lines in a plan or terminal.
func printable(s string) string {
	return strings.Map(func(r rune) rune {
		if r < 0x20 || r == 0x7f {
			return -1
		}
		return r
	}, s)
}

// scanReasons applies KILN03 6 to package p: its own files, and every symlink
// in them or in the reads.  It returns the reasons (never nil entries).
func (pl *planner) scanReasons(p int, members []int, items []readItem) ([]string, error) {
	var esc, ind, uns, links []string
	var allow *allowSet
	region := func() *allowSet {
		if allow == nil {
			allow = pl.newAllowSet(members, items)
		}
		return allow
	}
	for _, f := range pl.own[p] {
		r, err := pl.entryFull(f)
		if err != nil {
			return nil, err
		}
		if r.e.Kind == key.KindLink {
			links = append(links, f)
			continue
		}
		switch r.facts.state {
		case scanUnscanned:
			uns = append(uns, f)
		case scanText:
			if r.facts.indirect {
				ind = append(ind, f)
			}
			for _, t := range r.facts.refs {
				if !region().ok(t) {
					esc = append(esc, t)
				}
			}
			esc = append(esc, r.facts.dynamic...)
		}
	}
	for _, it := range items {
		ls, err := pl.linksOf(it)
		if err != nil {
			return nil, err
		}
		links = append(links, ls...)
	}
	for _, l := range links {
		res, err := pl.resolveLink(l)
		if err != nil {
			return nil, err
		}
		if !res.ok || !region().ok(res.final) {
			esc = append(esc, l)
		}
	}
	var out []string
	for _, k := range []struct {
		prefix string
		list   []string
	}{{"escape:", esc}, {"indirect:", ind}, {"unscanned:", uns}} {
		list := sortedKeys(toSet(k.list))
		if len(list) > maxScanReasons {
			list = list[:maxScanReasons]
		}
		for _, v := range list {
			out = append(out, k.prefix+printable(v))
		}
	}
	return out, nil
}

func toSet(xs []string) map[string]bool {
	m := make(map[string]bool, len(xs))
	for _, x := range xs {
		m[x] = true
	}
	return m
}

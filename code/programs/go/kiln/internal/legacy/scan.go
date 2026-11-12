package legacy

// Escape, indirection and unscanned rules (KILN03 section 6).
//
// A legacy BUILD can reach outside its closure in ways a key cannot see.  The
// closure and reads (KILN03 3.2, 3.3) are what the key covers; anything the
// files that run can point at beyond them is a hole.  Three cheap, conservative
// rules look for such pointers in EVERY file a package's result can depend on:
// its own files, every closure member's own files, and every read file.
//
//	../ resolution   a path token with a `..` segment, resolved against the
//	                 directory of the file it appears in, walked through any
//	                 symlinks, must land inside the package, a closure member
//	                 or a read path                                 escape:<path>
//	indirection      markers that compute a location at run time
//	                 (`dirname`, `$PWD`, `__file__`, `git rev-parse`, ...), a
//	                 `cd ..`, or an absolute path                   indirect:<file>
//	unscanned        a file the scanner cannot read as text: text over the
//	                 size cap, or non-text that is not a known inert image or
//	                 font format                                    unscanned:<file>
//
// They are heuristics, and every doubt is resolved toward a hit (a false
// positive costs cache hit rate; a false negative would be a false cache hit).
//
// The per-file part (what is in this file?) depends only on the file, so it is
// computed once, in the same pass that hashes the file (hashAndScan).  The
// per-package part (is that target inside THIS package's closure?) is done in
// scanReasons, from per-member and per-read summaries that are memoised.

import (
	"bytes"
	"hash"
	"io"
	"path"
	"regexp"
	"sort"
	"strings"
	"unicode/utf16"

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
	// scanSkipped: a non-executable file with the magic bytes of an inert
	// image, PDF or font format (inertMagics).  It is hashed but not scanned.
	scanSkipped scanState = iota
	// scanText: text within the cap; refs, dynamic and indirect are valid.
	scanText
	// scanUnscanned: text over the cap, or any other non-text file.
	scanUnscanned
)

// fileFacts is everything the per-file scan learns.
type fileFacts struct {
	state    scanState
	indirect bool     // a marker or an absolute path (KILN03 6.2)
	refs     []string // sorted, unique, repo-relative cleaned `..` targets; may start with "../" (leaves the repo)
	dynamic  []string // sorted, unique `..` tokens that cannot be resolved statically ($VAR, globs, ...)
}

// ---------------------------------------------------------------------------
// Text, UTF-16, inert formats
// ---------------------------------------------------------------------------

// inertMagics are the magic-byte prefixes of formats that cannot be run as a
// script by a BUILD: raster images, PDF and fonts.  A NON-executable file that
// starts with one of them is hashed and not scanned.  The list is deliberately
// short.  Residual risk, accepted and documented (KILN03 12): a script that
// begins with such a prefix and is run by `sh file.png` is not seen.
var inertMagics = [][]byte{
	[]byte("\x89PNG\r\n\x1a\n"),
	[]byte("\xff\xd8\xff"),             // JPEG
	[]byte("GIF87a"), []byte("GIF89a"), // GIF
	[]byte("\x00\x00\x01\x00"), // ICO
	[]byte("%PDF-"),
	[]byte("\x00\x01\x00\x00"), // TrueType
	[]byte("OTTO"),             // OpenType (CFF)
	[]byte("wOFF"), []byte("wOF2"),
}

type contentKind uint8

const (
	kindText   contentKind = iota // no NUL byte in the head: scanned as bytes, valid UTF-8 or not
	kindUTF16                     // starts with a UTF-16 byte order mark: decoded, then scanned
	kindInert                     // a known inert format
	kindBinary                    // anything else with a NUL byte
)

// kindOf decides what a file is from its head (its first 8 KiB, or all of it
// when shorter).  A UTF-16 BOM wins, then an inert magic, then the NUL test.
// Invalid UTF-8 without a NUL is still text: a script must not escape the scan
// by carrying one stray byte.
func kindOf(head []byte) contentKind {
	if len(head) >= 2 && (head[0] == 0xff && head[1] == 0xfe || head[0] == 0xfe && head[1] == 0xff) {
		return kindUTF16
	}
	for _, m := range inertMagics {
		if bytes.HasPrefix(head, m) {
			return kindInert
		}
	}
	if len(head) >= 12 && string(head[:4]) == "RIFF" && string(head[8:12]) == "WEBP" {
		return kindInert
	}
	if bytes.IndexByte(head, 0) >= 0 {
		return kindBinary
	}
	return kindText
}

// decodeUTF16 turns BOM-prefixed UTF-16 into a string; an odd trailing byte is
// dropped.
func decodeUTF16(b []byte) string {
	be := b[0] == 0xfe
	b = b[2:]
	u := make([]uint16, len(b)/2)
	for i := range u {
		if be {
			u[i] = uint16(b[2*i])<<8 | uint16(b[2*i+1])
		} else {
			u[i] = uint16(b[2*i+1])<<8 | uint16(b[2*i])
		}
	}
	return string(utf16.Decode(u))
}

// hashAndScan reads rc once: every byte goes to h (the content digest), and the
// first maxScanBytes are kept so they can be scanned.  The digest is exactly
// what io.Copy(h, rc) would produce.
//
//	head <= 8 KiB   decides the kind
//	inert           executable -> unscanned, otherwise skipped
//	binary          unscanned
//	text / UTF-16   <= cap: scanned;  > cap: unscanned (the rest is only hashed)
func hashAndScan(rc io.Reader, h hash.Hash, exec bool, dir string) (fileFacts, error) {
	head, err := io.ReadAll(io.LimitReader(rc, headBytes))
	if err != nil {
		return fileFacts{}, err
	}
	h.Write(head)
	kind := kindOf(head)
	if kind == kindInert || kind == kindBinary {
		if _, err := io.Copy(h, rc); err != nil {
			return fileFacts{}, err
		}
		if kind == kindInert && !exec {
			return fileFacts{state: scanSkipped}, nil
		}
		return fileFacts{state: scanUnscanned}, nil
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
	data := append(head, rest...)
	var s string
	if kind == kindUTF16 {
		s = decodeUTF16(data)
	} else {
		s = string(data)
	}
	f := scanContent(dir, s)
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
// `https://x/y` is not a file-system path, so it is removed before tokenising
// (KILN03 12).  Kept: `file:` URLs (they name a path) and any URL containing
// `..` (a `https://x/../y` is more likely a disguised path than a link).
var urlRE = regexp.MustCompile("(?i)[a-z][a-z0-9+.-]*://[^\\s\"'`<>()|;,]*")

func stripURLs(s string) string {
	if !strings.Contains(s, "://") {
		return s
	}
	return urlRE.ReplaceAllStringFunc(s, func(m string) string {
		if (len(m) >= 5 && strings.EqualFold(m[:5], "file:")) || strings.Contains(m, "..") {
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
//	".." "../" `..\`  -> the parent of the file's directory
//	"-I../inc"        -> a glued flag prefix is dropped: "../inc"
//	"a/.."            -> the trailing ".." is a segment too
//	"$ROOT/../x"      -> dynamic
//	"/a/../b"         -> neither: absolute paths are the indirection rule's
//	"wait..", "a..b"  -> neither: ".." must be a whole segment next to a "/"
func dotdot(dir, tok string) (target string, dynamic bool, ok bool) {
	t := strings.ReplaceAll(tok, `\`, "/")
	if t == ".." {
		t = "../"
	}
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

// markers are case-sensitive substrings.  Plain substring matching is the
// conservative choice: `dirname` also fires on `__dirname`, `os.path.dirname`
// and `mydirnamex`, which are exactly the shapes that matter, at the price of
// the odd false positive.
var markers = []string{
	// shell
	"dirname", "realpath", "readlink", "$0", "${0", "$BASH_SOURCE", "${BASH_SOURCE", "~/",
	"pushd", "Set-Location", "Push-Location", "chdir", "Chdir",
	// the file's own location, in other languages
	"__file__", "__dir__", "__FILE__", "__filename", "import.meta", "CARGO_MANIFEST_DIR",
	"MyInvocation", "PSCommandPath", "$PSScriptRoot",
	// the working directory, in other languages
	"getcwd", "Getwd", "cwd()", "Dir.pwd", "current_dir", "set_current_dir",
	"filepath.Dir", "filepath.Abs", "runtime.Caller", "Get-Location",
	// CI checkout locations
	"GITHUB_WORKSPACE", "RUNNER_",
	// Windows
	"%~dp0", "%CD%",
}

// ciMarkers are matched case-insensitively against a lower-cased copy
// (PowerShell variables are case-insensitive, and so is `%cd%`).
var ciMarkers = []string{
	"$(pwd", "`pwd`", "$pwd", "${pwd", "$oldpwd", "${oldpwd", "$home", "${home",
	"$psscriptroot", "%~dp0", "%cd%",
}

var (
	gitRevParseRE = regexp.MustCompile(`\bgit\s+rev-parse`)
	gitLsFilesRE  = regexp.MustCompile(`\bgit\s+ls-files`)
	// A bare `cd ..` (or `cd..`, `sl ..`): the word, optional whitespace,
	// optional quote, "..", any number of slashes, optional quote, then the end
	// of the argument.  `cd ../x` is not bare: it is a `../` token and is
	// judged by where it lands.  (`pushd`/`Set-Location`/`Push-Location`/
	// `chdir` are plain markers.)
	cdUpRE = regexp.MustCompile(`(?m)(?:^|[^A-Za-z0-9_.-])(?:cd|sl)[ \t]*["']?\.\.` + `/*["']?(?:[\s;&|)]|$)`)
)

// findMarkers lists the markers present in s (all of them, or just the first).
func findMarkers(s string, first bool) []string {
	var out []string
	add := func(m string) bool {
		out = append(out, m)
		return first
	}
	for _, m := range markers {
		if strings.Contains(s, m) && add(m) {
			return out
		}
	}
	if strings.ContainsAny(s, "$%`") {
		low := strings.ToLower(s)
		for _, m := range ciMarkers {
			if strings.Contains(low, m) && add(m) {
				return out
			}
		}
	}
	// The regexes are guarded by a cheap substring test: most files contain
	// none of these words, and Go's regexp engine is not fast.
	if strings.Contains(s, "rev-parse") && gitRevParseRE.MatchString(s) && add("git rev-parse") {
		return out
	}
	if strings.Contains(s, "ls-files") && gitLsFilesRE.MatchString(s) && add("git ls-files") {
		return out
	}
	if strings.Contains(s, "..") && cdUpRE.MatchString(s) {
		add("cd ..")
	}
	return out
}

func hasMarker(s string) bool { return len(findMarkers(s, true)) > 0 }

// allowedAbsolute are the absolute paths a BUILD may name without it being
// indirection (KILN03 6.2).
var allowedAbsolute = map[string]bool{"/dev/null": true, "/usr/bin/env": true, "/bin/sh": true}

// fsRoots are first path segments that name a real filesystem root, so a
// one-segment token such as `/tmp` is a path while `/p` or `/W4` is not.
var fsRoots = func() map[string]bool {
	m := map[string]bool{}
	for _, r := range strings.Fields("etc usr opt home Users tmp var private Library System Applications " +
		"Volumes mnt media proc sys dev root nix bin sbin lib lib64 snap srv run workspace github runner __w Windows") {
		m[r] = true
	}
	return m
}()

func isNameByte(c byte) bool {
	return c >= '0' && c <= '9' || c|0x20 >= 'a' && c|0x20 <= 'z' || c == '_' || c == '.' || c == '-'
}

// absoluteOutside reports whether a token is an absolute path outside the
// allowlist.  Rules, all biased toward reporting:
//
//   - a backslash counts as a slash, so `\usr\bin` and `\\host\share` are paths;
//   - a leading "#!" (a shebang) is ignored, and so is a leading run of
//     `- + @ [ {` (`${TMPDIR:-/tmp}` leaves the token `-/tmp}`, `@/etc/x`,
//     `[/etc`);
//   - a glued flag (`-I/usr/include`, `-isystem/usr/include`) counts as the
//     path after the flag name;
//   - `~name` (another user's home) is a path;
//   - a token made only of slashes is never a path (`//`, `///` comment leaders);
//   - a token containing ".." that starts with "/" is always reported;
//   - a token starting with ONE slash is a path when its first segment is a
//     well-known filesystem root (fsRoots), or when it has at least two
//     non-empty segments (`/a/b`).  `</p>` (the tag `/p`), `/W4` and
//     `/permissive-` have one segment and an unknown root, so they are not paths;
//   - a token starting with exactly TWO slashes is a path when, with the
//     slashes removed, it has at least two non-empty segments
//     (`//server/share`) or its first segment is a filesystem root (`//tmp`);
//     `//go:build` and `//TODO` are comments;
//   - three or more leading slashes (the tail of `file:///etc/x`) are judged as
//     the one-slash case on what follows them.
func absoluteOutside(tok string) bool {
	t := tok
	if strings.Contains(t, `\`) {
		t = strings.ReplaceAll(t, `\`, "/")
	}
	t = strings.TrimPrefix(t, "#!")
	flag := strings.HasPrefix(t, "-")
	t = strings.TrimLeft(t, "-+@[{(")
	if flag {
		// -I/usr/include, -isystem/usr/include: drop the flag name.
		for i := 0; i < len(t); i++ {
			if t[i] == '/' {
				t = t[i:]
				break
			}
			if !isNameByte(t[i]) {
				break
			}
		}
	}
	t = strings.TrimRight(t, "})]")
	if len(t) >= 2 && t[0] == '~' && (t[1]|0x20 >= 'a' && t[1]|0x20 <= 'z' || t[1] == '_') {
		return true
	}
	if len(t) < 2 || t[0] != '/' || allowedAbsolute[t] {
		return false
	}
	rest := strings.TrimLeft(t, "/")
	if rest == "" {
		return false
	}
	if strings.Contains(t, "..") {
		return true
	}
	lead := len(t) - len(rest)
	segs := 0
	first := ""
	for _, s := range strings.Split(rest, "/") {
		if s != "" {
			if segs == 0 {
				first = s
				for i := 0; i < len(first); i++ {
					if !isNameByte(first[i]) {
						first = first[:i] // `tmp}`, `etc*`
						break
					}
				}
			}
			segs++
		}
	}
	if lead == 2 {
		return segs >= 2 || fsRoots[first]
	}
	return fsRoots[first] || segs >= 2
}

// hasDrivePrefix reports a Windows drive path (`C:\x`, `C:/x`) or a UNC path
// (`\\host\share`) anywhere in s.  The tokeniser splits at `:` and would lose
// them.  A drive letter must not be glued to a longer word (`https://`).
func hasDrivePrefix(s string) bool {
	for i := 0; i+2 < len(s); i++ {
		if s[i] == ':' && (s[i+1] == '\\' || s[i+1] == '/') && i > 0 &&
			(s[i-1]|0x20 >= 'a' && s[i-1]|0x20 <= 'z') && (i == 1 || !isAlnum(s[i-2])) {
			return true
		}
	}
	for i := 0; ; {
		k := strings.Index(s[i:], `\\`)
		if k < 0 {
			return false
		}
		j := i + k + 2
		n := j
		for n < len(s) && (isAlnum(s[n]) || s[n] == '_' || s[n] == '.' || s[n] == '$' || s[n] == '-') {
			n++
		}
		if n > j && n < len(s) && s[n] == '\\' {
			return true
		}
		i = i + k + 1
	}
}

func isAlnum(c byte) bool { return c >= '0' && c <= '9' || c|0x20 >= 'a' && c|0x20 <= 'z' }

// scanContent is the per-file scan of a text file in directory dir.
func scanContent(dir, s string) fileFacts {
	var f fileFacts
	f.indirect = hasMarker(s) || hasDrivePrefix(s)
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
	pl    *planner
	dirs  map[string]bool // a target at or below any of these is inside
	globs []string        // read globs
}

func (pl *planner) newAllowSet(members []int, items []readItem) *allowSet {
	a := &allowSet{pl: pl, dirs: map[string]bool{}}
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
// target that climbs out of the repository never is.  A read glob admits a
// target only when the target is a listed file the glob matches, or a
// directory every one of whose files it matches.
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
		if a.pl.fileSet[target] {
			if source.Match(g, target) {
				return true
			}
			continue
		}
		under := a.pl.under(target)
		all := len(under) > 0
		for _, f := range under {
			if !source.Match(g, f) {
				all = false
				break
			}
		}
		if all {
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

// scanSummary is what a set of files contributes, independent of any package.
type scanSummary struct {
	ind, uns, refs, dyn, links []string
}

func (s *scanSummary) add(o scanSummary) {
	s.ind = append(s.ind, o.ind...)
	s.uns = append(s.uns, o.uns...)
	s.refs = append(s.refs, o.refs...)
	s.dyn = append(s.dyn, o.dyn...)
	s.links = append(s.links, o.links...)
}

// summarize folds the per-file facts of files into one summary.
func (pl *planner) summarize(files []string) (scanSummary, error) {
	var s scanSummary
	for _, f := range files {
		r, err := pl.entryFull(f)
		if err != nil {
			return s, err
		}
		if r.e.Kind == key.KindLink {
			s.links = append(s.links, f)
			continue
		}
		switch r.facts.state {
		case scanUnscanned:
			s.uns = append(s.uns, f)
		case scanText:
			if r.facts.indirect {
				s.ind = append(s.ind, f)
			}
			s.refs = append(s.refs, r.facts.refs...)
			s.dyn = append(s.dyn, r.facts.dynamic...)
		}
	}
	return s, nil
}

// scanReasons applies KILN03 6 to a package whose closure is members and whose
// reads are items.  The files examined are every member's own files and every
// read file; `../` targets (and symlinks) from ALL of them are judged against
// THIS package's region, so a dependency's escape or indirection reaches its
// dependents.  It returns the reasons.
func (pl *planner) scanReasons(members []int, items []readItem) ([]string, error) {
	var all scanSummary
	for _, q := range members {
		s, err := pl.memberSummary(q)
		if err != nil {
			return nil, err
		}
		all.add(s)
	}
	for _, it := range items {
		s, err := pl.itemSummary(it)
		if err != nil {
			return nil, err
		}
		all.add(s)
	}
	esc := append([]string{}, all.dyn...)
	var allow *allowSet
	region := func() *allowSet {
		if allow == nil {
			allow = pl.newAllowSet(members, items)
		}
		return allow
	}
	// Every ref target and every link is walked through any symlinks it
	// crosses (resolveLink) before the region check.
	for _, group := range [][]string{sortedKeys(toSet(all.refs)), sortedKeys(toSet(all.links))} {
		for _, x := range group {
			res, err := pl.resolveLink(x)
			if err != nil {
				return nil, err
			}
			if !res.ok || !region().ok(res.final) {
				esc = append(esc, x)
			}
		}
	}
	var out []string
	for _, k := range []struct {
		prefix string
		list   []string
	}{{"escape:", esc}, {"indirect:", all.ind}, {"unscanned:", all.uns}} {
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

// memberSummary is the summary of package q's own files (memoised).
func (pl *planner) memberSummary(q int) (scanSummary, error) {
	return pl.memberSums.get(pl.g.Packages[q].Name, func() (scanSummary, error) {
		return pl.summarize(pl.own[q])
	})
}

// itemSummary is the summary of every file a read item stands for (memoised).
func (pl *planner) itemSummary(it readItem) (scanSummary, error) {
	id := "p:" + it.path
	if it.glob {
		id = "g:" + it.path
	}
	return pl.itemSums.get(id, func() (scanSummary, error) {
		files, err := pl.itemFiles(it)
		if err != nil {
			return scanSummary{}, err
		}
		return pl.summarize(files)
	})
}

func toSet(xs []string) map[string]bool {
	m := make(map[string]bool, len(xs))
	for _, x := range xs {
		m[x] = true
	}
	return m
}

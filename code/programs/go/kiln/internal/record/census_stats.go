package record

// The census summary's small tools (KILN04, "`kiln record census`")
// =================================================================
//
// Everything here is pure: percentiles, log2 buckets, the text sanitiser, the
// redactor and the two Markdown quoting helpers.  They are kept apart from the
// directory walk (census_report.go) so each can be tested on its own, and so a
// reader meets the one dangerous idea of this feature in one place:
//
//	every string that came from a trace, a census file or a plan is
//	ATTACKER-INFLUENCED.  A build step chose the file names; a pull request
//	chose the build.  `traced.Sanitize` only strips non-printable runes.
//
// A path can therefore hold a newline, a backtick run, `|`, `<script>`, a bidi
// override, 5000 characters, or a line that starts with `::` (a GitHub workflow
// command when printed to a step's output).  The defences are layered:
//
//	safeText       non-printable runes become '?', redaction, a length cut
//	codeSpan       a path goes in a code span whose delimiter is longer than any
//	               backtick run inside it, so nothing in it can close the span
//	mdEscape       any other text gets the Markdown-special characters escaped
//	finishMarkdown a last pass: no control character, no line starting `::`
//
// and the JSON is produced by encoding/json, which escapes by itself.

import (
	"math"
	"math/bits"
	"sort"
	"strings"
	"unicode"
	"unicode/utf8"
)

// Dist is the shape of a distribution: how many samples and four order
// statistics (nearest rank).  An empty distribution is all zeros.
type Dist struct {
	N   int     `json:"n"`
	P50 float64 `json:"p50"`
	P90 float64 `json:"p90"`
	P99 float64 `json:"p99"`
	Max float64 `json:"max"`
}

// Bucket is one log2 bucket of a size histogram: Count samples in
// [Min, 2*Min), except Min 0, which holds the empty ones.
type Bucket struct {
	Min   int64 `json:"min"`
	Count int   `json:"count"`
}

// SizeDist is a Dist and its log2 histogram.
type SizeDist struct {
	Dist
	Buckets []Bucket `json:"log2_buckets"`
}

// percentile is the nearest-rank percentile p (1..100) of an ASCENDING slice:
// the smallest sample such that at least p percent of the samples are at or
// below it.  With n = 10 and p = 90 that is the 9th sample; with n = 1 every
// percentile is that one sample.  An empty slice gives 0.
func percentile(sorted []float64, p int) float64 {
	n := len(sorted)
	if n == 0 {
		return 0
	}
	rank := (p*n + 99) / 100 // ceil(p*n/100), in integers
	if rank < 1 {
		rank = 1
	}
	if rank > n {
		rank = n
	}
	return sorted[rank-1]
}

// round3 keeps three decimals, so the JSON does not carry float noise.  It also
// refuses NaN and infinities (encoding/json cannot encode them): they become 0.
func round3(f float64) float64 {
	if math.IsNaN(f) || math.IsInf(f, 0) {
		return 0
	}
	return math.Round(f*1000) / 1000
}

// distOf summarises vals (which it does not modify).
func distOf(vals []float64) Dist {
	if len(vals) == 0 {
		return Dist{}
	}
	s := append([]float64(nil), vals...)
	sort.Float64s(s)
	return Dist{
		N:   len(s),
		P50: round3(percentile(s, 50)), P90: round3(percentile(s, 90)),
		P99: round3(percentile(s, 99)), Max: round3(s[len(s)-1]),
	}
}

// log2Buckets counts sizes into power-of-two buckets: a value v lands in the
// bucket whose Min is the largest power of two not above v (0 for v == 0).
// Only non-empty buckets are returned, ascending.
//
//	v = 0          -> Min 0
//	v = 1          -> Min 1        (bits.Len64(1) = 1, Min = 1<<0)
//	v = 1000       -> Min 512      (bits.Len64(1000) = 10, Min = 1<<9)
//	v = 1024       -> Min 1024
func log2Buckets(sizes []int64) []Bucket {
	counts := map[int]int{}
	for _, v := range sizes {
		if v < 0 {
			v = 0
		}
		counts[bits.Len64(uint64(v))]++
	}
	ks := make([]int, 0, len(counts))
	for k := range counts {
		ks = append(ks, k)
	}
	sort.Ints(ks)
	out := make([]Bucket, 0, len(ks))
	for _, k := range ks {
		var min int64
		if k > 0 {
			min = 1 << (k - 1)
		}
		out = append(out, Bucket{Min: min, Count: counts[k]})
	}
	return out
}

// sizeDistOf is distOf and log2Buckets of one list of byte counts.
func sizeDistOf(sizes []int64) SizeDist {
	f := make([]float64, len(sizes))
	for i, v := range sizes {
		f[i] = float64(v)
	}
	return SizeDist{Dist: distOf(f), Buckets: log2Buckets(sizes)}
}

// Redaction replaces one directory prefix (From) in any text with a label.
type Redaction struct{ From, To string }

// redactor applies redactions, longest From first, so a path under both
// $HOME and $HOME/work is labelled by the more specific one.
type redactor struct{ rs []Redaction }

// newRedactor keeps only the redactions that can be meaningful: an absolute
// path of at least two characters ("/" would label every slash).  A trailing
// slash is dropped, so "/home/u/" and "/home/u" are the same.
func newRedactor(in []Redaction) *redactor {
	seen := map[string]bool{}
	var rs []Redaction
	for _, r := range in {
		from := strings.TrimRight(r.From, "/")
		if len(from) < 2 || from[0] != '/' || seen[from] {
			continue
		}
		seen[from] = true
		rs = append(rs, Redaction{From: from, To: r.To})
	}
	sort.SliceStable(rs, func(i, j int) bool { return len(rs[i].From) > len(rs[j].From) })
	return &redactor{rs: rs}
}

// apply redacts s.  A nil redactor changes nothing.  A prefix is replaced only
// at a path-component boundary (replaceAtBoundary), so "/tmp" never touches
// "/usr/lib/tmpfiles.d" or "/var/tmp".
func (r *redactor) apply(s string) string {
	if r == nil {
		return s
	}
	for _, x := range r.rs {
		if strings.Contains(s, x.From) {
			s = replaceAtBoundary(s, x.From, x.To)
		}
	}
	return s
}

// isPathByte says b can be part of a path component: a letter, a digit, one of
// "_.-", a slash, or any byte of a multi-byte character.
func isPathByte(b byte) bool {
	return b >= 0x80 || b == '/' || b == '_' || b == '.' || b == '-' || b >= '0' && b <= '9' || b >= 'a' && b <= 'z' || b >= 'A' && b <= 'Z'
}

// replaceAtBoundary replaces the occurrences of from (a directory path) in s
// that are whole path prefixes: the match must begin s or follow a character
// that cannot be part of a path (a space, '=', a quote), and must be followed by
// '/', the end of s, or such a character.  So with from "/tmp":
//
//	/tmp/x            replaced          ($TMPDIR/x)
//	a=/tmp            replaced          (a=$TMPDIR)
//	/var/tmp/y        kept  (the match follows "r", a path character)
//	/usr/lib/tmpfiles.d/x   kept  (the match follows "b" and precedes "f")
//	/tmpx             kept  (the match precedes "x")
func replaceAtBoundary(s, from, to string) string {
	var b strings.Builder
	i := 0
	for {
		k := strings.Index(s[i:], from)
		if k < 0 {
			break
		}
		k += i
		end := k + len(from)
		before := k == 0 || !isPathByte(s[k-1])
		after := end == len(s) || s[end] == '/' || !isPathByte(s[end])
		if before && after {
			b.WriteString(s[i:k])
			b.WriteString(to)
			i = end
		} else {
			b.WriteString(s[i : k+1])
			i = k + 1
		}
	}
	b.WriteString(s[i:])
	return b.String()
}

// safeText makes untrusted text printable, redacted and short.  Invalid UTF-8
// and every non-printable rune (a control character, a bidi override, a line
// separator) become '?'; then redaction; then the text is cut to max runes (the
// cut is marked with "...", and truncated is true).
func safeText(s string, max int, r *redactor) (out string, truncated bool) {
	s = strings.ToValidUTF8(s, "?")
	s = strings.Map(func(c rune) rune {
		if !unicode.IsPrint(c) {
			return '?'
		}
		return c
	}, s)
	s = r.apply(s)
	if utf8.RuneCountInString(s) > max {
		i, n := 0, 0
		for i < len(s) && n < max {
			_, w := utf8.DecodeRuneInString(s[i:])
			i += w
			n++
		}
		return s[:i] + "...", true
	}
	return s, false
}

// pathPrefix is the first depth components of a path, with the leading slash
// kept: "/usr/lib/x86_64-linux-gnu/libc.so" at depth 4 is
// "/usr/lib/x86_64-linux-gnu/libc.so" and at depth 2 "/usr/lib".  Empty
// components ("//", a trailing slash) are dropped, so spellings of one path
// group together.  The empty path is "(empty)".
func pathPrefix(p string, depth int) string {
	abs := strings.HasPrefix(p, "/")
	var parts []string
	for _, c := range strings.Split(p, "/") {
		if c != "" {
			parts = append(parts, c)
			if len(parts) == depth {
				break
			}
		}
	}
	switch {
	case len(parts) == 0 && abs:
		return "/"
	case len(parts) == 0:
		return "(empty)"
	case abs:
		return "/" + strings.Join(parts, "/")
	}
	return strings.Join(parts, "/")
}

// mdSpecial are the characters mdEscape puts a backslash before.  The first
// eleven are the ones the census contract names; "~" is strikethrough in GFM.
const mdSpecial = "\\`|<>[]&*_#~"

// mdEscape quotes text for Markdown prose (not for a code span).
func mdEscape(s string) string {
	var b strings.Builder
	for _, c := range s {
		if strings.ContainsRune(mdSpecial, c) {
			b.WriteByte('\\')
		}
		b.WriteRune(c)
	}
	return b.String()
}

// codeSpan wraps s in a Markdown code span that s cannot escape.  The delimiter
// is a backtick run one longer than the longest run in s, so no run inside s can
// close it; a text that begins or ends with a backtick (or is padded with
// spaces) gets one space of padding on each side, which Markdown strips.  Inside
// a code span nothing else is special, so `<script>` and `|` stay literal text
// (a `|` still breaks a TABLE cell, which is why attacker text never goes in
// one).
func codeSpan(s string) string {
	if s == "" {
		return "(empty)"
	}
	longest, run := 0, 0
	for _, c := range s {
		if c == '`' {
			run++
			if run > longest {
				longest = run
			}
		} else {
			run = 0
		}
	}
	d := strings.Repeat("`", longest+1)
	if strings.HasPrefix(s, "`") || strings.HasSuffix(s, "`") || (strings.HasPrefix(s, " ") && strings.HasSuffix(s, " ")) {
		return d + " " + s + " " + d
	}
	return d + s + d
}

// finishMarkdown is the last defence on the whole document: any control
// character other than newline becomes '?', and a line that starts with "::"
// (a GitHub workflow command, if the text ever reached a step's output instead
// of its summary file) gets a backslash in front.  By construction no line of
// ours starts that way; this pass is what makes it hold whatever a later edit
// does.
func finishMarkdown(s string) string {
	s = strings.ToValidUTF8(s, "?")
	s = strings.Map(func(c rune) rune {
		if c != '\n' && (unicode.IsControl(c) || c == ' ' || c == ' ') {
			return '?'
		}
		return c
	}, s)
	lines := strings.Split(s, "\n")
	for i, l := range lines {
		if strings.HasPrefix(l, "::") {
			lines[i] = "\\" + l
		}
	}
	return strings.Join(lines, "\n")
}

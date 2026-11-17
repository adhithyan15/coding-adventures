package trace

import (
	"errors"
	"strings"
	"testing"
)

// TestDefaultPathIsTheGrammarPath pins the D24 PR 6 flip (KILN04 section 4.9):
// with the cross-check hook UNSET (what production runs), Parse reads lines
// with the grammar path.  Two inputs tell the readers apart:
//
//   - "[pid +5] ..." (a signed pid): the old scanner ACCEPTS it, the grammar
//     path refuses it.  If the default were still the old scanner, Parse would
//     succeed.
//   - "stat(a) = 0 <unfinished ...>": both refuse, but with different reasons
//     (old: result; grammar: unparsable-line), so the reason names the reader.
//
// With the hook set, the grammar path's answer is still the one returned.
func TestDefaultPathIsTheGrammarPath(t *testing.T) {
	saved := crossCheck
	defer func() { crossCheck = saved }()

	cases := []struct{ name, in, want string }{
		{"signed pid (stricter-only: old accepts)", "[pid +5] stat(\"/a\", 0x1) = 0\n", reasonUnparsable},
		{"bare marker after the close (reason-only)", "1  stat(\"/a\", 0x1) = 0 <unfinished ...>\n", reasonUnparsable},
	}
	for _, c := range cases {
		for _, hook := range []bool{false, true} {
			crossCheck = nil
			if hook {
				crossCheck = parseCrossChecked
			}
			_, err := Parse(strings.NewReader(c.in), "/w", Limits{})
			var te *Error
			if !errors.As(err, &te) || te.Reason != c.want {
				t.Errorf("%s (cross-check %v): got %v, want reason %s", c.name, hook, err, c.want)
			}
		}
		// The oracle, called directly, must differ: otherwise the case does not
		// tell the two readers apart.
		_, err := parseWith(strings.NewReader(c.in), "/w", Options{}, false)
		var te *Error
		if err != nil && errors.As(err, &te) && te.Reason == c.want {
			t.Errorf("%s: the oracle gave the grammar reason too; the case does not tell the readers apart", c.name)
		}
	}
}

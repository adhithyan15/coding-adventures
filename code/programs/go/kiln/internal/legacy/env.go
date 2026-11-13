package legacy

import (
	"crypto/sha256"
	"fmt"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/envscrub"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
)

// EnvPolicy is a validated env.json (KILN03 5): the names of the variables
// whose values can change a build or test result.
type EnvPolicy struct {
	exact    map[string]bool
	prefixes []string
	// Digest is SHA-256 of the file's bytes: the `policy:env.json` consumed
	// entry of the static key.
	Digest [32]byte
}

type envFile struct {
	Names []string `json:"names"`
}

// secretMarkers are the substrings (compared case-insensitively) that make an
// environment variable name secret-like (KILN03 5).  `AUTH` is deliberately
// blunt: it also catches GIT_AUTHOR_NAME, and over-matching only costs a
// variable the key does not need, where a miss would put a brute-forceable
// digest of a credential in a plan.
var secretMarkers = []string{"TOKEN", "SECRET", "PASSWORD", "PASSWD", "CREDENTIAL", "AUTH", "PRIVATE"}

// secretLike reports whether a variable name looks like it holds a secret:
// KILN03 5's rule, plus everything the child-process scrubber already removes
// (so the two lists can never disagree about what a credential is).
func secretLike(name string) bool {
	n := strings.ToUpper(name)
	for _, m := range secretMarkers {
		if strings.Contains(n, m) {
			return true
		}
	}
	return strings.HasSuffix(n, "_KEY") || strings.HasSuffix(n, "_PAT") || envscrub.IsSecretName(name)
}

// envNameOK is the spelling of an allowed variable name: letters, digits and
// underscore, not starting with a digit.
func envNameOK(n string) bool {
	if n == "" || len(n) > 128 {
		return false
	}
	for i := 0; i < len(n); i++ {
		c := n[i]
		if !(c == '_' || c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || i > 0 && c >= '0' && c <= '9') {
			return false
		}
	}
	return true
}

// ParseEnvPolicy reads env.json strictly.  A name containing ':' is an error
// (those are reserved for Kiln: `kiln:root`, `kiln:recorder`, ... in the KILN04
// weak key), and so is a name, literal or prefix, that is secret-like.  A
// trailing `*` makes a prefix; a bare `*` would allow everything and is refused.
func ParseEnvPolicy(data []byte) (*EnvPolicy, error) {
	var f envFile
	if err := strictDecode(data, &f); err != nil {
		return nil, fmt.Errorf("env: %w", err)
	}
	if err := checkSorted("env names", f.Names); err != nil {
		return nil, err
	}
	p := &EnvPolicy{exact: map[string]bool{}, Digest: sha256.Sum256(data)}
	for _, n := range f.Names {
		if strings.Contains(n, ":") {
			return nil, fmt.Errorf("env: name %q contains ':', which is reserved for Kiln", short(n))
		}
		stem, prefix := strings.CutSuffix(n, "*")
		if !envNameOK(stem) {
			return nil, fmt.Errorf("env: invalid name %q", short(n))
		}
		if secretLike(stem) {
			return nil, fmt.Errorf("env: %q is secret-like and may not be keyed", short(n))
		}
		if prefix {
			p.prefixes = append(p.prefixes, stem)
		} else {
			p.exact[stem] = true
		}
	}
	return p, nil
}

// Names lists the policy's names as written (prefixes keep their `*`).
func (p *EnvPolicy) Names() []string {
	out := make([]string, 0, len(p.exact)+len(p.prefixes))
	for n := range p.exact {
		out = append(out, n)
	}
	for _, pre := range p.prefixes {
		out = append(out, pre+"*")
	}
	sort.Strings(out)
	return out
}

func (p *EnvPolicy) allows(name string) bool {
	if p.exact[name] {
		return true
	}
	for _, pre := range p.prefixes {
		if strings.HasPrefix(name, pre) {
			return true
		}
	}
	return false
}

// EnvFor is the `env` section of a key (KILN03 5): every variable of environ
// ("NAME=value" entries) the policy allows, with its value, sorted by name.  An
// unset variable is simply absent, an empty one stays an empty value, so
// the two remain distinct.  A variable that matches only after wildcard
// expansion and is secret-like is an ERROR, not a silent skip; an invalid
// UTF-8 value is an error too (a key cannot hold it).  The first of two
// entries with one name wins, as getenv would.  Names are compared exactly.
func (p *EnvPolicy) EnvFor(environ []string) ([]key.EnvVar, error) {
	seen := map[string]bool{}
	var out []key.EnvVar
	for _, e := range environ {
		name := envscrub.EntryName(e)
		if seen[name] || !p.allows(name) {
			continue
		}
		seen[name] = true
		if secretLike(name) {
			return nil, fmt.Errorf("env: variable %q is set, matches env.json and looks like a secret; unset it or narrow env.json", short(name))
		}
		value := strings.TrimPrefix(e[len(name):], "=")
		if !utf8.ValidString(value) {
			return nil, fmt.Errorf("env: variable %q holds a value that is not valid UTF-8", short(name))
		}
		out = append(out, key.EnvVar{Name: name, Value: value, Origin: key.OriginEnv})
	}
	sort.Slice(out, func(a, b int) bool { return out[a].Name < out[b].Name })
	return out, nil
}

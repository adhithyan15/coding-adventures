package lexer

// Compile cache for the legacy constructor
// ========================================
//
// NewGrammarLexer(source, grammar) is called once per source text by existing
// code, often with a tiny source. Compiling the grammar on every call would
// make that path slower than it ever was, so the constructor reuses a *Compiled
// per *TokenGrammar pointer.
//
// A cache keyed by pointer is only safe if a grammar mutated after first use is
// never served stale. Each entry therefore stores a fingerprint of everything
// Compile reads (rule counts, names, patterns, flags, keyword lists, modes and
// transitions); a mismatch recompiles and replaces the entry. The fingerprint
// is a 64-bit FNV-1a hash, cheap compared with compiling and good enough here:
// the failure it guards against is an accidental edit, not an adversary.
//
// The cache is bounded: when it holds cacheLimit entries it is emptied, so a
// program that builds many throwaway grammars cannot grow it without limit or
// pin them in memory forever.
//
// Compile / Compiled.Tokenize do not use the cache; callers who hold a
// *Compiled manage its lifetime themselves.

import (
	"hash/fnv"
	"sort"
	"strconv"
	"sync"
	"sync/atomic"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

const cacheLimit = 128

type cacheEntry struct {
	fingerprint uint64
	compiled    *Compiled
}

var (
	compileCache     sync.Map // *grammartools.TokenGrammar -> *cacheEntry
	compileCacheSize atomic.Int64
)

// ForgetGrammar drops any cached compilation of tg. Call it when a grammar is
// discarded to release it early (the cache also empties itself when it fills).
func ForgetGrammar(tg *grammartools.TokenGrammar) {
	if _, loaded := compileCache.LoadAndDelete(tg); loaded {
		compileCacheSize.Add(-1)
	}
}

// compiledFor returns a cached *Compiled for tg, compiling if there is none or
// the grammar has changed since it was cached. Errors are never cached.
func compiledFor(tg *grammartools.TokenGrammar) (*Compiled, error) {
	if tg == nil {
		return Compile(tg)
	}
	fp := fingerprint(tg)
	if v, ok := compileCache.Load(tg); ok {
		if e := v.(*cacheEntry); e.fingerprint == fp {
			return e.compiled, nil
		}
	}
	c, err := Compile(tg)
	if err != nil {
		return nil, err
	}
	if compileCacheSize.Load() >= cacheLimit {
		compileCache.Range(func(k, _ any) bool {
			compileCache.Delete(k)
			return true
		})
		compileCacheSize.Store(0)
	}
	if _, loaded := compileCache.Swap(tg, &cacheEntry{fp, c}); !loaded {
		compileCacheSize.Add(1)
	}
	return c, nil
}

// fingerprint hashes every part of the grammar that Compile consumes.
func fingerprint(tg *grammartools.TokenGrammar) uint64 {
	h := fnv.New64a()
	w := func(s string) {
		h.Write([]byte(strconv.Itoa(len(s))))
		h.Write([]byte{':'})
		h.Write([]byte(s))
	}
	defs := func(label string, ds []grammartools.TokenDefinition) {
		w(label)
		w(strconv.Itoa(len(ds)))
		for _, d := range ds {
			w(d.Name)
			w(d.Pattern)
			w(d.Alias)
			if d.IsRegex {
				w("r")
			} else {
				w("l")
			}
		}
	}
	list := func(label string, xs []string) {
		w(label)
		w(strconv.Itoa(len(xs)))
		for _, x := range xs {
			w(x)
		}
	}
	defs("defs", tg.Definitions)
	defs("skip", tg.SkipDefinitions)
	names := make([]string, 0, len(tg.Groups))
	for n := range tg.Groups {
		names = append(names, n)
	}
	sort.Strings(names)
	for _, n := range names {
		defs("group "+n, tg.Groups[n].Definitions)
	}
	list("kw", tg.Keywords)
	list("reserved", tg.ReservedKeywords)
	list("context", tg.ContextKeywords)
	list("layout", tg.LayoutKeywords)
	w(tg.Mode)
	w(tg.EscapeMode)
	w(tg.StartMode)
	flags := "cs"
	if tg.CaseSensitive {
		flags = "CS"
	}
	if tg.CaseInsensitive {
		flags += "CI"
	}
	w(flags)
	w(strconv.Itoa(len(tg.Transitions)))
	for _, t := range tg.Transitions {
		list("on", t.OnTokens)
		w(t.OnValue)
		w(t.InMode)
		for _, a := range t.Actions {
			w(a.Kind)
			w(a.Target)
		}
	}
	return h.Sum64()
}

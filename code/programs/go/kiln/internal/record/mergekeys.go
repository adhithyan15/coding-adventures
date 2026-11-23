package record

// The merge job's keys (KILN04 section 7.1, G4; stage 4 PR 3a)
// =========================================================================
//
// The merge job "computes every digest and strong key itself, from `Source`,
// and never trusts a digest from a shard". This file is that promise in code:
// `recmerge.Keys`, implemented over `legacy.Traced`, which is the same
// implementation `prepare` records with and `legacy-plan` plans with. There is
// one key algorithm in this repository and this type only adapts it.
//
// Why an adapter at all? A shard's records arrive with the *machine facts* its
// runner observed (image id, checkout path, `$HOME`, the scrubbed environment),
// and the weak key is a function of exactly those. So the merge rebuilds one
// `*legacy.Traced` per DISTINCT facts value, not one per action: a six-hourly
// full build has thousands of actions but at most a handful of distinct
// (image, root, home, environment) tuples, and `legacy.Traced` memoises every
// digest it computes. Rebuilding it per action would re-hash the tree for
// every record.
//
// Why not trust the shard's own weak key? Because a shard could name any key
// it liked, and a wrong-but-well-formed weak key is exactly how a shard would
// smuggle a false hit past the merge. The merge recomputes W from the shard's
// facts and refuses the shard when the two differ (B3).

import (
	"errors"
	"fmt"
	"sync"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// TracedKeysOptions is what a TracedKeys needs to build its Traced values.
type TracedKeysOptions struct {
	// Graph is the build tool's export at the merge's own revision.
	Graph *legacy.Graph
	// Source reads the merged tree. Required.
	Source legacy.Source
	// Root is the checkout path the plan was made for.
	Root string
	// ExecutorArgs are the build tool's flags that change what a BUILD does,
	// in the canonical form `recmerge.Run` was recorded with. Nil means
	// `legacy.RecordExecutorArgs`.
	ExecutorArgs []string
	// Arch is the machine architecture in KILN02 spelling. Empty means the
	// running machine's.
	Arch string
	// newTraced builds one Traced from options; it exists so a test can count
	// how many were built (the memo's whole point) without hashing a tree.
	newTraced func(*legacy.Graph, legacy.Options, string, []string) (Traced, error)
}

// Traced is the part of *legacy.Traced this adapter uses. It is an interface so
// a test can hold the memo's contract (and the refusal path) without hashing a
// tree; the only production implementation is *legacy.Traced.
type Traced interface {
	WeakKey(rel string) (*legacy.WeakKey, error)
	StrongKey(w *legacy.WeakKey, names []string) (*legacy.StrongKey, error)
}

// TracedKeys implements recmerge.Keys.
//
// It is safe for concurrent use. Two actions with the same facts share one
// Traced; two with different facts (two runner images, say) get one each, and
// never see each other's memo.
type TracedKeys struct {
	graph *legacy.Graph
	src   legacy.Source
	root  string
	args  []string
	arch  string
	make  func(*legacy.Graph, legacy.Options, string, []string) (Traced, error)

	mu    sync.Mutex
	cache map[string]Traced // by the canonical facts bytes
	built int
}

// NewTracedKeys prepares the adapter. The architecture is resolved here, once,
// so an unsupported machine fails before any shard is judged.
func NewTracedKeys(o TracedKeysOptions) (*TracedKeys, error) {
	if o.Graph == nil {
		return nil, errors.New("a graph is required: the keys are computed from it")
	}
	if o.Source == nil {
		return nil, errors.New("a Source is required: the keys are computed from it")
	}
	if o.Root == "" {
		return nil, errors.New("a root is required: the weak key names it")
	}
	arch := o.Arch
	if arch == "" {
		a, err := legacy.HostArch()
		if err != nil {
			return nil, err
		}
		arch = a
	}
	args := o.ExecutorArgs
	if args == nil {
		args = legacy.RecordExecutorArgs
	}
	mk := o.newTraced
	if mk == nil {
		mk = func(g *legacy.Graph, opt legacy.Options, root string, args []string) (Traced, error) {
			return legacy.NewTraced(g, opt, root, args)
		}
	}
	return &TracedKeys{
		graph: o.Graph, src: o.Source, root: o.Root,
		args: append([]string(nil), args...), arch: arch,
		make: mk, cache: map[string]Traced{},
	}, nil
}

// Weak is the weak key of action under the shard's own facts. A refusal (a
// package with no traced key: an unknown toolchain, a symlink out of the
// closure, ...) is returned as its reason, not as an error, because it is a
// verdict about the action and the merge charges it to the action.
func (k *TracedKeys) Weak(f recfmt.HostFacts, action string) ([32]byte, string, error) {
	tr, err := k.traced(f)
	if err != nil {
		return [32]byte{}, "", err
	}
	w, err := tr.WeakKey(action)
	if err != nil {
		var ref *legacy.Refusal
		if errors.As(err, &ref) {
			return [32]byte{}, ref.Reason, nil
		}
		return [32]byte{}, "", err
	}
	return w.Key, "", nil
}

// Strong is the strong key of action for the manifest's entry names, each
// digested as it is NOW in the merge's verified tree -- which is the whole
// point of recomputing here (KILN04 section 3.2).
func (k *TracedKeys) Strong(f recfmt.HostFacts, action string, names []string) ([32]byte, error) {
	tr, err := k.traced(f)
	if err != nil {
		return [32]byte{}, err
	}
	w, err := tr.WeakKey(action)
	if err != nil {
		return [32]byte{}, err
	}
	s, err := tr.StrongKey(w, names)
	if err != nil {
		return [32]byte{}, err
	}
	return s.Key, nil
}

// Built reports how many Traced values were constructed, which is how a test
// holds the memo's contract (one per distinct facts, not one per call).
func (k *TracedKeys) Built() int {
	k.mu.Lock()
	defer k.mu.Unlock()
	return k.built
}

// traced returns the one Traced for these facts, building it on first use.
//
// The cache key is the canonical encoding of the facts, which `recfmt`
// already defines and which is total (every field is covered), so two facts
// values that differ in anything cannot share a Traced. The facts are also
// what the weak key is made from, so a cache HIT here means the two shards
// agree about the machine, which is the only case in which sharing is right.
func (k *TracedKeys) traced(f recfmt.HostFacts) (Traced, error) {
	key, err := recfmt.EncodeHostFacts(f)
	if err != nil {
		return nil, fmt.Errorf("host facts: %w", err)
	}
	k.mu.Lock()
	defer k.mu.Unlock()
	if tr, ok := k.cache[string(key)]; ok {
		return tr, nil
	}
	tr, err := k.make(k.graph, legacy.Options{
		Source:  k.src,
		Host:    f,
		OS:      "linux",
		Arch:    k.arch,
		Environ: f.Environ(),
	}, k.root, k.args)
	if err != nil {
		return nil, err
	}
	k.cache[string(key)] = tr
	k.built++
	return tr, nil
}

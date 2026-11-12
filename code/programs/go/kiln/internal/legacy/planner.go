package legacy

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"path"
	"runtime"
	"sort"
	"strings"
	"sync"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// Source is what legacy planning needs from the tree: a file listing and file
// contents (source.Source) plus how each entry is stored (source.Describer).
// Both source.Git and source.Folder qualify.
type Source interface {
	source.Source
	source.Describer
}

// Imager is the part of a Host that planning needs: the runner image id that
// goes into every key's host section.
type Imager interface{ ImageID() string }

// Options configure one planning run.
type Options struct {
	Source  Source
	Host    Imager
	OS      string // linux | darwin | windows (the graph's spelling)
	Arch    string // x64 | arm64 (KILN02 spelling)
	Workers int    // hashing parallelism; 0 means a default bounded by CPU count
}

// keyOS maps the graph's OS name to KILN02's.
func keyOS(osName string) string {
	if osName == "darwin" {
		return "macos"
	}
	return osName
}

// HostArch returns the running machine's architecture in KILN02 spelling.
func HostArch() (string, error) {
	switch runtime.GOARCH {
	case "amd64":
		return "x64", nil
	case "arm64":
		return "arm64", nil
	}
	return "", fmt.Errorf("unsupported architecture %q", runtime.GOARCH)
}

// cell memoises one computation; concurrent callers share a single run.
type cell[V any] struct {
	once sync.Once
	v    V
	err  error
}

// memo is a keyed set of cells: each key's function runs at most once per
// planning run ("each digest is computed once per run", KILN03 3.4).
type memo[V any] struct {
	mu sync.Mutex
	m  map[string]*cell[V]
}

func (m *memo[V]) get(k string, f func() (V, error)) (V, error) {
	m.mu.Lock()
	if m.m == nil {
		m.m = map[string]*cell[V]{}
	}
	c, ok := m.m[k]
	if !ok {
		c = &cell[V]{}
		m.m[k] = c
	}
	m.mu.Unlock()
	c.once.Do(func() { c.v, c.err = f() })
	return c.v, c.err
}

// readItem is one thing a closure member reads: a path (a file or a
// directory, decided by what the tree holds) or a glob pattern.
type readItem struct {
	glob bool
	path string
}

// resolved is a read after looking at the tree: either one file entry for
// `reads`, or one consumed tree digest.
type resolved struct {
	file     *key.FileEntry
	consumed *key.Consumed
}

// planner holds one run's indexes and memo tables.
type planner struct {
	g   *Graph
	opt Options
	pol *policy

	files   []string // every Source file, sorted
	fileSet map[string]bool
	byName  map[string]int
	byPath  map[string]int

	own       [][]string // files each package owns (deepest-owner rule)
	nested    [][]int    // packages strictly under p
	enclosing [][]int    // packages strictly enclosing p
	deps      [][]int    // direct deps on the chosen OS
	dev       [][]int    // dev_deps
	cfgFiles  map[string][]string
	bounds    []boundary
	items     [][]readItem

	entries   memo[entryResult]
	trees     memo[[32]byte]
	resolves  memo[resolved]
	itemFs    memo[[]string]       // files behind a read item
	itemLinks memo[[]string]       // symlinks among them
	linkRes   memo[linkResolution] // where a symlink finally points
	linkInfo  memo[source.Info]    // Describe of a link, once
	ownDigest [][32]byte

	toolOnce sync.Once
	tool     [32]byte
	toolErr  error
}

// workers returns the pool size: at most the caller's choice, else the CPU
// count, never below one and never above 32.
func (o Options) workers() int {
	n := o.Workers
	if n <= 0 {
		n = runtime.NumCPU()
	}
	return max(1, min(n, 32))
}

// forEach runs fn(0..n-1) on a bounded pool and returns the first error (the
// remaining work is abandoned).  Results are written by index, so scheduling
// order never reaches the output.
func forEach(n, workers int, fn func(i int) error) error {
	var (
		wg   sync.WaitGroup
		mu   sync.Mutex
		next int
		fail error
	)
	for w := 0; w < min(workers, n); w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for {
				mu.Lock()
				if fail != nil || next >= n {
					mu.Unlock()
					return
				}
				i := next
				next++
				mu.Unlock()
				if err := fn(i); err != nil {
					mu.Lock()
					if fail == nil {
						fail = err
					}
					mu.Unlock()
					return
				}
			}
		}()
	}
	wg.Wait()
	return fail
}

// newPlanner indexes the graph against the tree.
func newPlanner(g *Graph, opt Options) (*planner, error) {
	pl := &planner{g: g, opt: opt}
	files, err := opt.Source.Files("", nil)
	if err != nil {
		return nil, err
	}
	pl.files = files
	pl.fileSet = make(map[string]bool, len(files))
	for _, f := range files {
		pl.fileSet[f] = true
	}
	n := len(g.Packages)
	pl.byName = make(map[string]int, n)
	pl.byPath = make(map[string]int, n)
	for i, p := range g.Packages {
		pl.byName[p.Name] = i
		pl.byPath[p.RelPath] = i
	}
	pl.nested = make([][]int, n)
	pl.enclosing = make([][]int, n)
	for i, p := range g.Packages {
		for d := p.RelPath; ; {
			cut := strings.LastIndexByte(d, '/')
			if cut < 0 {
				break
			}
			d = d[:cut]
			if j, ok := pl.byPath[d]; ok {
				pl.enclosing[i] = append(pl.enclosing[i], j)
				pl.nested[j] = append(pl.nested[j], i)
			}
		}
	}
	// Deepest-owner rule: a file belongs to the package whose directory is the
	// nearest ancestor that is a package.  That is exactly "under rel_path,
	// excluding nested packages' subtrees" (KILN03 3.1); files with no owner
	// feed the ancestor-configuration rule.
	pl.own = make([][]string, n)
	pl.cfgFiles = map[string][]string{}
	for _, f := range files {
		if i, ok := pl.ownerOf(f); ok {
			pl.own[i] = append(pl.own[i], f)
			continue
		}
		dir := ""
		if cut := strings.LastIndexByte(f, '/'); cut >= 0 {
			dir = f[:cut]
		}
		pl.cfgFiles[dir] = append(pl.cfgFiles[dir], f)
	}
	pl.deps = make([][]int, n)
	pl.dev = make([][]int, n)
	for i, p := range g.Packages {
		for _, d := range p.Deps[opt.OS] {
			pl.deps[i] = append(pl.deps[i], pl.byName[d])
		}
		for _, d := range p.DevDeps {
			pl.dev[i] = append(pl.dev[i], pl.byName[d])
		}
	}
	return pl, nil
}

// ownerOf finds the deepest package directory above file f.
func (pl *planner) ownerOf(f string) (int, bool) {
	for d := f; ; {
		cut := strings.LastIndexByte(d, '/')
		if cut < 0 {
			return 0, false
		}
		d = d[:cut]
		if i, ok := pl.byPath[d]; ok {
			return i, true
		}
	}
}

// under lists the files at or below dir (dir itself if it is a file).  The
// Source list is bytewise sorted and "/" sorts before every name byte that
// could follow a shared prefix, so the matches are one contiguous run.
func (pl *planner) under(dir string) []string {
	if pl.fileSet[dir] {
		return []string{dir}
	}
	prefix := dir + "/"
	i := sort.SearchStrings(pl.files, prefix)
	j := i
	for j < len(pl.files) && strings.HasPrefix(pl.files[j], prefix) {
		j++
	}
	return pl.files[i:j]
}

// closure is KILN03 3.2: the least set containing {p} and dev_deps(p), closed
// under deps, N (nested) and E (enclosing) for EVERY member.  dev_deps are
// seeded once, at p.  It is a set, so cycles are harmless; the result is
// sorted, so the order of discovery never matters.
func (pl *planner) closure(p int) []int {
	in := map[int]bool{}
	work := []int{p}
	in[p] = true
	for _, d := range pl.dev[p] {
		if !in[d] {
			in[d] = true
			work = append(work, d)
		}
	}
	for len(work) > 0 {
		q := work[len(work)-1]
		work = work[:len(work)-1]
		for _, rel := range [][]int{pl.deps[q], pl.nested[q], pl.enclosing[q]} {
			for _, r := range rel {
				if !in[r] {
					in[r] = true
					work = append(work, r)
				}
			}
		}
	}
	out := make([]int, 0, len(in))
	for q := range in {
		out = append(out, q)
	}
	sort.Ints(out)
	return out
}

// entryResult is what reading one listed path yields: its KILN02 inputs entry
// and, for a regular file that some package owns, the scan facts of KILN03 6,
// gathered in the SAME pass over the bytes that produced the digest.
type entryResult struct {
	e     key.FileEntry
	facts fileFacts
}

// entry builds the KILN02 inputs entry of one listed path: kind, executable
// bit (from the Source, never the disk) and content digest (a link's digest
// covers its target TEXT only).  Memoised, so a file shared by many reads is
// hashed once.
func (pl *planner) entry(p string) (key.FileEntry, error) {
	r, err := pl.entryFull(p)
	return r.e, err
}

// entryFull is entry plus the scan facts.  Whether facts are gathered depends
// only on the path (does some package own it?), never on who asked first, so
// the memoised result is the same whichever caller gets there first.
func (pl *planner) entryFull(p string) (entryResult, error) {
	return pl.entries.get(p, func() (entryResult, error) {
		info, err := pl.opt.Source.Describe(p)
		if err != nil {
			return entryResult{}, err
		}
		h := sha256.New()
		if info.Link {
			h.Write([]byte(info.Target))
			return entryResult{e: key.FileEntry{Path: p, Kind: key.KindLink, Digest: h.Sum(nil)}}, nil
		}
		rc, err := pl.opt.Source.Open(p)
		if err != nil {
			return entryResult{}, err
		}
		defer rc.Close()
		var facts fileFacts
		if _, owned := pl.ownerOf(p); owned {
			facts, err = hashAndScan(rc, h, info.Exec, path.Dir(p))
		} else {
			_, err = io.Copy(h, rc)
		}
		if err != nil {
			return entryResult{}, fmt.Errorf("%s: %w", short(p), err)
		}
		e := key.FileEntry{Path: p, Kind: key.KindFile, Digest: h.Sum(nil)}
		if info.Exec {
			e.Exec = 1
		}
		return entryResult{e: e, facts: facts}, nil
	})
}

// entriesOf maps sorted paths to their entries.
func (pl *planner) entriesOf(paths []string) ([]key.FileEntry, error) {
	out := make([]key.FileEntry, 0, len(paths))
	for _, p := range paths {
		e, err := pl.entry(p)
		if err != nil {
			return nil, err
		}
		out = append(out, e)
	}
	return out, nil
}

// treeDigest is the digest of a set of files: SHA-256 of the KILN02 inputs
// encoding of every one of them, so paths, kinds, executable bits AND content
// digests all count, never paths alone (KILN03 3.3).
func (pl *planner) treeDigest(paths []string) ([32]byte, error) {
	es, err := pl.entriesOf(paths)
	if err != nil {
		return [32]byte{}, err
	}
	return key.FilesDigest("inputs", es)
}

// computeOwn fills ownDigest for every package, in parallel.
func (pl *planner) computeOwn() error {
	pl.ownDigest = make([][32]byte, len(pl.g.Packages))
	return forEach(len(pl.g.Packages), pl.opt.workers(), func(i int) error {
		d, err := pl.treeDigest(pl.own[i])
		if err != nil {
			return fmt.Errorf("package %s: %w", short(pl.g.Packages[i].Name), err)
		}
		pl.ownDigest[i] = d
		return nil
	})
}

// ignoredConfig reports whether an ancestor-directory file is on the ignore
// list of KILN03 3.3 item 3 (a deliberate hit-rate choice).
func ignoredConfig(name string) bool {
	return strings.HasSuffix(name, ".md") || strings.HasPrefix(name, "LICENSE") ||
		strings.HasPrefix(name, "CHANGELOG") || name == ".gitignore"
}

// memberItems lists what package q itself reads (KILN03 3.3): its repo_inputs
// and globs, the boundary inputs that apply to it, and the configuration files
// sitting directly in each ancestor directory of its rel_path.
func (pl *planner) memberItems() {
	pl.items = make([][]readItem, len(pl.g.Packages))
	for i, p := range pl.g.Packages {
		set := map[readItem]bool{}
		for _, in := range p.RepoInputs {
			set[readItem{path: in}] = true
		}
		for _, gl := range p.RepoInputGlobs {
			set[readItem{glob: true, path: gl}] = true
		}
		for _, b := range pl.bounds {
			if b.appliesTo(p.RelPath) {
				for _, in := range b.inputs {
					set[readItem{path: in}] = true
				}
			}
		}
		for d := p.RelPath; ; {
			cut := strings.LastIndexByte(d, '/')
			if cut < 0 {
				d = ""
			} else {
				d = d[:cut]
			}
			for _, f := range pl.cfgFiles[d] {
				if !ignoredConfig(path.Base(f)) {
					set[readItem{path: f}] = true
				}
			}
			if d == "" {
				break
			}
		}
		for it := range set {
			pl.items[i] = append(pl.items[i], it)
		}
		sort.Slice(pl.items[i], func(a, b int) bool {
			x, y := pl.items[i][a], pl.items[i][b]
			return x.glob != y.glob && !x.glob || x.glob == y.glob && x.path < y.path
		})
	}
}

// resolve turns a read item into a reads entry or a consumed tree digest.
//
//	a path that is a listed file      -> one `reads` entry
//	a path that is a directory        -> consumed "dir:<path>" = tree digest
//	a path that matches nothing       -> consumed "dir:<path>" = digest of the
//	                                     EMPTY tree, so the path appearing
//	                                     later changes the key
//	a glob                            -> consumed "glob:<pattern>" = tree digest
//	                                     of every file the Source matches
func (pl *planner) resolve(it readItem) (resolved, error) {
	id := "p:" + it.path
	if it.glob {
		id = "g:" + it.path
	}
	return pl.resolves.get(id, func() (resolved, error) {
		if !it.glob && pl.fileSet[it.path] {
			e, err := pl.entry(it.path)
			return resolved{file: &e}, err
		}
		files, err := pl.itemFiles(it)
		if err != nil {
			return resolved{}, err
		}
		d, err := pl.treeDigest(files)
		if err != nil {
			return resolved{}, err
		}
		target := "dir:" + it.path
		if it.glob {
			target = "glob:" + it.path
		}
		return resolved{consumed: &key.Consumed{Target: target, Phase: key.PhaseBuild, Key: d[:]}}, nil
	})
}

// itemFiles lists the files a read item stands for: the file itself, every
// file under a directory, or every file a glob matches.  Memoised, because the
// link check asks again for every package that reads the item.
func (pl *planner) itemFiles(it readItem) ([]string, error) {
	id := "p:" + it.path
	if it.glob {
		id = "g:" + it.path
	}
	return pl.itemFs.get(id, func() ([]string, error) {
		if !it.glob {
			return pl.under(it.path), nil
		}
		var hits []string
		for _, f := range pl.files {
			if source.Match(it.path, f) {
				hits = append(hits, f)
			}
		}
		return hits, nil
	})
}

// buildToolDigest is the tool:build-tool consumed digest, computed once.
func (pl *planner) buildToolDigest() ([32]byte, error) {
	pl.toolOnce.Do(func() {
		files, err := pl.buildToolFiles()
		if err != nil {
			pl.toolErr = err
			return
		}
		pl.tool, pl.toolErr = pl.treeDigest(files)
	})
	return pl.tool, pl.toolErr
}

func hexOf(b []byte) string { return hex.EncodeToString(b) }

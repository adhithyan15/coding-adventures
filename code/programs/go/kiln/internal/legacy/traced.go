package legacy

// The traced legacy keys (KILN04 sections 3, 4.2 and 5.2): the weak key that
// selects manifests, the strong key that the pass ledger stores, and the digest
// provider that supplies "each manifest entry as it is now".
//
// The ENCODING lives in internal/key (key/traced.go): state digests, listing
// digests, T(q), and the strong key's consumed section.  This file decides
// WHICH values to encode, from the tracked tree (Source), the legacy graph and
// the policy files.  Nothing here reads the disk directly and nothing runs a
// BUILD.
//
// # The picture
//
//	            weak key W                 strong key S
//	  platform  linux x64                  (same)
//	  host      ImageID                    (same)
//	  action    (rel_path, test)           (same)
//	  recipe    the BUILD this OS runs     (same)
//	  inputs    empty                      (same)
//	  reads     empty                      (same)
//	  consumed  tool:build-tool            weak:      -> W
//	            policy:env.json            pkg:<q>    -> T(q)         (per manifest
//	            policy:host-roots.json     path:<p>   -> state digest  entry, as it
//	            policy:git                 list:<d>   -> listing digest is today)
//	  tools     pins of the static closure + the always set   (same)
//	  env       the scrubbed environment + kiln:* entries     (same)
//
// W covers what a system-call trace cannot see.  S adds what the trace saw.
// Only S has a `weak:` entry, so the two can never be equal.

import (
	"crypto/sha256"
	"errors"
	"fmt"
	"io"
	"path"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"sync"
	"unicode"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/envscrub"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
)

// TracedSemantics is the KILN02 semantics version of every traced key.
// Version 0 is the static legacy key and 1 is reserved for KILN01.
const TracedSemantics uint64 = 2

// RecorderVersion is the `kiln:recorder` entry of the weak key: an integer
// bumped BY HAND on any change to KILN04 sections 4 to 7 (what is recorded,
// how it is parsed, what is refused).  Bumping it changes every weak key, so
// manifests recorded by the old recorder are never looked up by the new one.
// It is a constant in code, not a file, so a change to it is reviewed together
// with the recorder change it accompanies.
//
// Version 3: strace now prints every signal (-e signal=all, D19), so a process
// killed by a signal has an exit line, and the daemon-left rule keys on the
// init's exit_group call line.  Both change what a trace means.
//
// Version 4: a refused-class call the sandbox filter answered (clone3 = -1
// ENOSYS, the others = -1 EPERM) is tolerated instead of refused, and a clone
// with a forbidden flag that failed with EPERM likewise.  Traces that were
// refused now parse, so every weak key changes.
//
// Version 5: the dying-call rule (KILN04 D35).  A task that dies inside a system
// call when its thread group exits is tolerated in two of its three spellings
// (`= ? <unavailable>`, and `<... clone resumed> <unfinished ...>) = ?`, with an
// unfinished call followed only by the task's exit) when the call is ignored,
// fork, vfork, or a clone with readable and allowed flags.  A call that reads is
// still refused.  Traces that were refused now parse, so every weak key changes.
//
// Version 6: the recipe prefix rule (KILN04 D36).  An action that failed before
// its last step has fewer step traces than recipe lines; when they are a byte
// exact prefix of the recipe and the last one exited non-zero, that is no longer
// `trace-incomplete:recipe-mismatch`.  The verdict for those actions changes, so
// every weak key changes.
//
// Version 7: the ancestor-node and fresh-tmpfs-absence rules (KILN04 D39).  A
// present, non-listing look at a proper ancestor of the checkout, a declared
// root, Home, Tmpdir or an ephemeral or absent-ok entry, and an absent probe
// strictly inside the fresh Home or Tmpdir, are no longer `host-read`.  The
// verdict for those actions changes, so every weak key changes.
//
// Version 8: tmpfs containment (KILN04 D40).  While the action has made no
// symbolic link inside the fresh Home or Tmpdir, the stability rules do not
// apply to paths there (its own scratch files are re-created, unlinked and
// renamed freely), so such actions are no longer `trace-incomplete:unstable-path`
// nor writes-incomplete.  The verdict changes, so every weak key changes.
//
// Version 9: descriptor metadata writes on anonymous objects (KILN04 D41).
// fchmod, fchown, fsetxattr and fremovexattr on a descriptor whose -y text is
// pipe:[..], socket:[..] or anon_inode:.. are no longer `trace-incomplete:fd-path`;
// they record nothing.  The accept set grows, so every weak key changes.
//
// Version 10: absent-before-made (KILN04 D42).  A lookup that failed with ENOENT
// on a name the action created afterwards, a metadata write by path (chmod,
// chown, utimensat, setxattr) on a file the action made, and shell redirections
// to /dev/null are no longer `trace-incomplete:unstable-path`.  The accept set
// grows, so every weak key changes.
//
// Version 11: the sealed root (KILN04 D43).  An absent probe of a path the
// sandbox's new root does not contain (/run/systemd/resolve/stub-resolv.conf,
// the package.json above the checkout, /var/run) is no longer `host-read`, and
// the scratch directories (/tmp, /var/tmp, /run) are fresh tmpfs like Home and
// Tmpdir.  New refusals too: a call that SUCCEEDED where the plan has nothing,
// and any absent probe allowed here or by D39 inside a scratch directory after
// the action moved a directory.  The verdict changes both ways, so every weak
// key changes.
//
// Version 12: more dying calls and orphans accepted (KILN04 D44).  A dying
// stat, lstat, access or readlink with a NULL path is dropped; a pid whose
// only line is its exit is accepted when dropped dying spawns can account for
// it; mknod/mknodat without a device argument (regular file, FIFO, socket) is
// no longer `arity`.  The accept set grows, so every weak key changes.
//
// Version 13: completed NULL-path statx failing with EFAULT (KILN04 D45).  A
// completed statx with a literal NULL path, valid flags without AT_EMPTY_PATH,
// and a -1 EFAULT result is dropped with no event (e.g. Rust std's statx probe).
// The accept set grows, so every weak key changes.
//
// Version 14: thread orphans (KILN04 D44a).  When every dying spawn the D35 rule
// dropped is a clone with CLONE_THREAD, the orphan exits D44 accepts are threads
// and no longer count as possible survivors, so a step that ends in a
// `daemon-left` refusal today can now be accepted.  The verdict changes, so
// every weak key changes.
//
// Version 15: dying unknown-number calls (KILN04 D47).  A dying call that strace
// spells `syscall_0x...` is dropped when its number is above the sandbox
// filter's ceiling (the filter answered ENOSYS; internal/syscallceil) and fits
// in 32 bits, or is 456 (futex_requeue, no path).  Go actions that were refused
// as `unresumed-call` by a dying `syscall_0x1c8` can now be accepted.  The
// accept set grows, so every weak key changes.
//
// Version 16: cwd-race by overlap (KILN04 D46).  A chdir or fchdir on a cwd
// shared with another live process (threads, CLONE_FS) is accepted unless a
// sharer's cwd-dependent call overlaps its line interval, instead of being
// refused whenever a sharer exists.  A sharer call that overlaps a chdir used
// to be seen only as "a sharer exists"; the same overlap is now also checked
// when the chdir's sharer was already gone, and a copying fork or clone and an
// unshare(CLONE_FS), every exec, a relative AF_UNIX address and an AT_FDCWD
// empty path count as cwd-dependent calls.  The verdict changes both
// ways, so every weak key changes.
//
// Version 17: the exec listener and the PR_SET_MM refusal (KILN04 D48, step 3).
// The sandbox's first filter now answers prctl(PR_SET_MM, ...) (option 35, any
// sub-option, high bits of the option ignored) with EPERM, which changes what a
// step can do; and `sandbox-exec` gained the optional --exec-notify-fd, with
// which the init installs a second filter that notifies every execve and
// execveat.  Nothing the classifier believes changes yet (consumption is step 5,
// which bumps again), but the same action could in principle record differently,
// so every weak key changes.
//
// Version 18 was skipped: it was reserved for KILN04 D46a (#19908), which
// merged after this one and took 20.
//
// Version 19: exec heads recorded at exec time are BELIEVED (KILN04 D48, step 5).
// An exec of a program under the step's fresh tmpfs (the `go test` binary) is no
// longer always `exec-unreadable:under-tmp`: when the step ran with `--exec-head`
// and its `.exechead` file pairs with the trace (one line per exec event of each
// pid, a `confirmed` status, the same path bytes, a kernel series on the table),
// the loader chain is read from the head the supervisor took from the pinned
// inode.  The accept set grows, so every weak key changes.  (Taking the number:
// main was 17 when this was written; if #19908 has not merged this is main+2, and
// whichever of the two lands second renumbers.)
//
// Version 20: a split
// helper-window unshare(CLONE_FS) is checked over its own line interval
// instead of being refused whenever another thread shares the cwd (KILN04
// D46a).  The sandbox helper's unshare is split whenever another Go thread's
// stop is handled between its two halves, which made the structure probes
// fail at random.  Traces that were refused now parse, so every weak key
// changes.
//
// Version 21: two parse shapes are accepted that were refused (KILN04 D50).  A
// fork, vfork or clone that ended `= ? ERESTARTNOINTR` created nothing and is
// dropped; a successful read-only open or openat with no -y annotation is
// recorded against the walked path.  The accept set grows, so every weak key
// changes.  (20 is D46a, #19908.)
const RecorderVersion = 21

// Names of the weak key's Kiln-owned env entries.  A ':' makes a name reserved
// for Kiln (env.json may not list one), so none can collide with a real
// variable.  They are keyed but never passed to the traced process.
const (
	EnvRoot         = "kiln:root"
	EnvRecorder     = "kiln:recorder"
	EnvExecutorArgs = "kiln:executor-args"
)

// Refusal means "no traced key for this package", for a reason the planner
// reports (`host-unknown`, `unknown-toolchain:<name>`, `no-build-file:<os>`,
// `symlink:<file>`).  It is not a malfunction: the package simply always runs.
type Refusal struct{ Reason string }

func (r *Refusal) Error() string { return "no traced key: " + r.Reason }

// WeakKey is the weak key and everything it was made from.
type WeakKey struct {
	// Action is the KILN02 action: semantics version 2, empty inputs and reads.
	Action key.Action
	// Key is its key.
	Key [32]byte
	// Scrubbed is the environment the traced run must have, as variables
	// (sorted by name), so the recorder runs with exactly what is keyed.
	Scrubbed []key.EnvVar
}

// Environ renders the scrubbed environment as "NAME=value" entries for
// os/exec's Cmd.Env.
func (w *WeakKey) Environ() []string { return EnvironOf(w.Scrubbed) }

// StrongKey is a strong key with the digests it holds.
type StrongKey struct {
	Action  key.Action
	Key     [32]byte
	Entries []key.EntryDigest
}

// Strong derives the strong key from the weak key and the manifest's entries,
// each already paired with its digest (KILN04 section 3.2).  Every section but
// `consumed` is the weak key's; `consumed` is `weak:` -> W plus one entry per
// manifest entry.  The `tool:` and `policy:` entries are not repeated: they
// reach S through W.
func (w *WeakKey) Strong(entries []key.EntryDigest) (*StrongKey, error) {
	consumed, err := key.StrongConsumed(w.Key, entries)
	if err != nil {
		return nil, err
	}
	a := w.Action.Clone() // the strong key shares nothing with the weak one
	a.Consumed = consumed
	k, err := key.Key(a)
	if err != nil {
		return nil, err
	}
	return &StrongKey{Action: a, Key: k, Entries: append([]key.EntryDigest(nil), entries...)}, nil
}

// Traced computes weak keys, strong keys and current entry digests for one
// tracked tree.  It is safe for concurrent use; every digest is computed once.
type Traced struct {
	pl    *planner
	root  string
	args  string
	weak  memo[*WeakKey]
	own   memo[[32]byte]
	state memo[[32]byte]
	gitD  memo[[32]byte]
	// prep runs the preparation Gate needs (gate.go) once.
	prep    sync.Once
	prepErr error
}

// CanonicalExecutorArgs is the `kiln:executor-args` value: the executor's flags
// in one canonical string.  Each flag is one whole token (`-name=value`, never
// `-name value`), the tokens are sorted bytewise, each is Go-quoted, and they
// are joined by single spaces, so the same flags given in another order key
// the same.  Two flags with one name are refused: sorting would otherwise throw
// away which of them the executor sees last.
func CanonicalExecutorArgs(flags []string) (string, error) {
	toks := append([]string(nil), flags...)
	sort.Strings(toks)
	names := map[string]bool{}
	for _, f := range toks {
		if !utf8.ValidString(f) || len(f) < 2 || f[0] != '-' {
			return "", fmt.Errorf("executor flag %q is not a -name[=value] token", short(f))
		}
		name, _, _ := strings.Cut(strings.TrimLeft(f, "-"), "=")
		if name == "" || names[name] {
			return "", fmt.Errorf("executor flag %q is empty or given twice", short(f))
		}
		names[name] = true
	}
	quoted := make([]string, len(toks))
	for i, f := range toks {
		quoted[i] = strconv.Quote(f)
	}
	return strings.Join(quoted, " "), nil
}

// NewTraced indexes the graph against the tree and loads the policy files.
// root is the absolute checkout path (`kiln:root`); flags are the executor's
// flags (see CanonicalExecutorArgs).  opt.Environ is the process environment
// the scrubbed environment is derived from.
func NewTraced(g *Graph, opt Options, root string, flags []string) (*Traced, error) {
	if opt.Source == nil || opt.Host == nil {
		return nil, fmt.Errorf("traced key: source and host are required")
	}
	known := false
	for _, o := range OSes {
		known = known || o == opt.OS
	}
	if !known {
		return nil, fmt.Errorf("traced key: unknown os %q", short(opt.OS))
	}
	if opt.Arch != "x64" && opt.Arch != "arm64" {
		return nil, fmt.Errorf("traced key: unknown arch %q", short(opt.Arch))
	}
	args, err := checkRootAndArgs(root, flags)
	if err != nil {
		return nil, err
	}
	pl, err := newPlanner(g, opt)
	if err != nil {
		return nil, err
	}
	if err := pl.checkGraphMatchesSource(); err != nil {
		return nil, err
	}
	names := make(map[string]bool, len(g.Packages))
	for _, p := range g.Packages {
		names[p.Name] = true
	}
	if pl.pol, err = loadPolicy(opt.Source, names); err != nil {
		return nil, err
	}
	return &Traced{pl: pl, root: root, args: args}, nil
}

// checkRootAndArgs validates `kiln:root` and canonicalises the executor flags.
func checkRootAndArgs(root string, flags []string) (string, error) {
	if !filepath.IsAbs(root) || !utf8.ValidString(root) || strings.ContainsFunc(root, unicode.IsControl) {
		return "", fmt.Errorf("traced key: checkout root %q must be an absolute path", short(root))
	}
	return CanonicalExecutorArgs(flags)
}

// checkGraphMatchesSource is a PARTIAL guard against a stale graph: every BUILD
// file the graph names must exist in the tree.  T(q) takes its nested-package
// names from the graph, so a graph exported from another tree could let a new
// nested package leave T(q) unchanged.  A stray BUILD elsewhere is not checked:
// the old tool deliberately does not treat those (test fixtures, say) as
// packages, so they are ordinary files of their enclosing package.  The real
// requirement, that the graph come from the same tree as Source, is KILN04
// 11.1 item 11.
func (pl *planner) checkGraphMatchesSource() error {
	for _, p := range pl.g.Packages {
		for _, os := range OSes {
			if bf := p.BuildFiles[os]; bf != nil && !pl.fileSet[p.RelPath+"/"+*bf] {
				return fmt.Errorf("traced key: graph package %s names %s, which is not in the tree (stale graph?)", short(p.Name), short(*bf))
			}
		}
	}
	return nil
}

// ScrubbedEnv is the environment of a traced run (KILN04 section 4.2), as the
// key's env variables, sorted by name.  It is built in this order, a later
// step replacing an earlier one of the same name:
//
//  1. the variables env.json allows, with their values (EnvPolicy.EnvFor);
//  2. HOME, and TMPDIR if set, taken from environ;
//  3. PATH, CONSTRUCTED from the roots of exactly tools (never inherited);
//  4. LANG=C.UTF-8 and TZ=UTC, which override env.json.
//
// HOME is passed when set and left out when not, so the run sees exactly what
// is keyed.  A name containing ':' is refused: those are Kiln's.
func ScrubbedEnv(pol *EnvPolicy, tc *Toolchains, tools []key.Tool, environ []string) ([]key.EnvVar, error) {
	allowed, err := pol.EnvFor(environ)
	if err != nil {
		return nil, err
	}
	vars := map[string]key.EnvVar{}
	for _, v := range allowed {
		vars[foldEnv(v.Name)] = v
	}
	set := func(name, value string) {
		vars[foldEnv(name)] = key.EnvVar{Name: name, Value: value, Origin: key.OriginEnv}
	}
	lookup := func(name string) (string, bool) {
		val, ok := "", false
		for _, e := range environ {
			if foldEnv(envscrub.EntryName(e)) == foldEnv(name) {
				val, ok = strings.TrimPrefix(e[len(envscrub.EntryName(e)):], "="), true // the LAST entry wins
			}
		}
		return val, ok
	}
	home, haveHome := lookup("HOME")
	if haveHome {
		set("HOME", home)
	} else {
		delete(vars, foldEnv("HOME")) // env.json cannot conjure one either
	}
	if v, ok := lookup("TMPDIR"); ok {
		set("TMPDIR", v)
	} else {
		delete(vars, foldEnv("TMPDIR"))
	}
	set("PATH", tc.ConstructPATH(tools, home))
	set("LANG", "C.UTF-8")
	set("TZ", "UTC")
	out := make([]key.EnvVar, 0, len(vars))
	for _, v := range vars {
		if strings.Contains(v.Name, ":") {
			return nil, fmt.Errorf("scrubbed env: name %q contains ':', which is reserved for Kiln", short(v.Name))
		}
		if !utf8.ValidString(v.Value) {
			return nil, fmt.Errorf("scrubbed env: variable %q holds a value that is not valid UTF-8", short(v.Name))
		}
		out = append(out, v)
	}
	sort.Slice(out, func(a, b int) bool { return out[a].Name < out[b].Name })
	return out, nil
}

// EnvironOf renders variables as "NAME=value" entries.
func EnvironOf(vars []key.EnvVar) []string {
	out := make([]string, len(vars))
	for i, v := range vars {
		out[i] = v.Name + "=" + v.Value
	}
	return out
}

// pkgAt finds a package by its rel_path.
func (t *Traced) pkgAt(rel string) (int, bool) {
	i, ok := t.pl.byPath[rel]
	return i, ok
}

// WeakKey computes the weak key of the package whose rel_path is rel (KILN04
// section 3.1).  A package that cannot be traced gets a *Refusal.
func (t *Traced) WeakKey(rel string) (*WeakKey, error) {
	p, ok := t.pkgAt(rel)
	if !ok {
		return nil, fmt.Errorf("traced key: no package at %q", short(rel))
	}
	w, err := t.weak.get(rel, func() (*WeakKey, error) { return t.weakKey(p) })
	if err != nil {
		return nil, err
	}
	return w.clone(), nil // callers get their own copy, never the memoised one
}

// clone deep-copies a weak key.
func (w *WeakKey) clone() *WeakKey {
	return &WeakKey{Action: w.Action.Clone(), Key: w.Key, Scrubbed: append([]key.EnvVar(nil), w.Scrubbed...)}
}

func (t *Traced) weakKey(p int) (*WeakKey, error) {
	pl := t.pl
	pkg := pl.g.Packages[p]
	image := pl.opt.Host.ImageID()
	// A self-hosted runner's image id carries no identity, so it can neither
	// record nor look up (KILN04 3.1).
	if strings.Contains(image, "unknown") {
		return nil, &Refusal{Reason: "host-unknown"}
	}
	recipe, why, err := pl.buildRecipe(pkg)
	if err != nil {
		return nil, err
	}
	if why != "" {
		return nil, &Refusal{Reason: why}
	}
	// The tools are those of the STATIC closure (KILN03 3.2): the traced run
	// may read more, and a read under a tool root whose tool is absent from
	// here is a recorder refusal (`host-read:tool-not-keyed`), not a key input.
	tools, unknown := pl.pol.tc.ToolsFor(pl.closureNames(pl.closure(p)), pl.opt.OS, image)
	if len(unknown) > 0 {
		return nil, &Refusal{Reason: "unknown-toolchain:" + unknown[0]}
	}
	td, err := pl.buildToolDigest()
	if err != nil {
		return nil, err
	}
	gd, err := t.gitDigest()
	if err != nil {
		return nil, err
	}
	scrubbed, err := ScrubbedEnv(pl.pol.env, pl.pol.tc, tools, pl.opt.Environ)
	if err != nil {
		return nil, err
	}
	env := append([]key.EnvVar(nil), scrubbed...)
	env = append(env,
		key.EnvVar{Name: EnvRoot, Value: t.root, Origin: key.OriginEnv},
		key.EnvVar{Name: EnvRecorder, Value: strconv.Itoa(RecorderVersion), Origin: key.OriginEnv},
		key.EnvVar{Name: EnvExecutorArgs, Value: t.args, Origin: key.OriginEnv})
	sort.Slice(env, func(a, b int) bool { return env[a].Name < env[b].Name })
	a := key.Action{
		SemanticsVersion: TracedSemantics,
		OS:               keyOS(pl.opt.OS), Arch: pl.opt.Arch,
		HostImageID: &image,
		Target:      pkg.RelPath, Phase: key.PhaseTest,
		Recipe: recipe,
		Consumed: key.SortedConsumed([]key.Consumed{
			{Target: "tool:build-tool", Phase: key.PhaseBuild, Key: td[:]},
			{Target: "policy:env.json", Phase: key.PhaseBuild, Key: pl.pol.env.Digest[:]},
			{Target: "policy:host-roots.json", Phase: key.PhaseBuild, Key: pl.pol.roots.Digest[:]},
			{Target: "policy:git", Phase: key.PhaseBuild, Key: gd[:]},
		}),
		Tools: tools,
		Env:   env,
	}
	k, err := key.Key(a)
	if err != nil {
		return nil, err
	}
	return &WeakKey{Action: a, Key: k, Scrubbed: scrubbed}, nil
}

// gitDigest is `policy:git`: the digest of every .gitattributes and
// .gitmodules file in the tree, as KILN02 `inputs` entries (path, kind,
// executable bit, content).  Git reads them while checking out, which no trace
// sees; a changed attribute can change the bytes a BUILD finds.  No such file
// at all is the digest of the empty list, so adding the first one changes it.
func (t *Traced) gitDigest() ([32]byte, error) {
	return t.gitD.get("git", func() ([32]byte, error) {
		var paths []string
		for _, f := range t.pl.files {
			if b := path.Base(f); b == ".gitattributes" || b == ".gitmodules" {
				paths = append(paths, f)
			}
		}
		return t.pl.treeDigest(paths)
	})
}

// StrongKey computes the strong key for weak key w and the manifest entries
// names, each digested as it is now (CurrentDigests).
func (t *Traced) StrongKey(w *WeakKey, names []string) (*StrongKey, error) {
	ds, err := t.CurrentDigests(names)
	if err != nil {
		return nil, err
	}
	return w.Strong(ds)
}

// CurrentDigests is the digest provider of lookup (stage 5) and the merge job:
// for each entry name, its digest in the TRACKED tree now.  Names are checked
// against the manifest allowlist first.  Results come in the order asked.
func (t *Traced) CurrentDigests(names []string) ([]key.EntryDigest, error) {
	out := make([]key.EntryDigest, len(names))
	for i, n := range names {
		d, err := t.EntryDigest(n)
		if err != nil {
			return nil, err
		}
		out[i] = key.EntryDigest{Name: n, Digest: d}
	}
	return out, nil
}

// EntryDigest is one entry's current digest, computed once per Traced.
func (t *Traced) EntryDigest(name string) ([32]byte, error) {
	if err := key.CheckEntryName(name); err != nil {
		return [32]byte{}, err
	}
	return t.state.get(name, func() ([32]byte, error) {
		switch {
		case strings.HasPrefix(name, key.EntryPackage):
			return t.packageDigest(name[len(key.EntryPackage):])
		case strings.HasPrefix(name, key.EntryPath):
			return t.pathDigest(name[len(key.EntryPath):])
		default:
			return t.listDigest(name[len(key.EntryList):])
		}
	})
}

// ownDigest is D(q) of package q (KILN03 3.1), computed once.
func (t *Traced) ownDigest(q int) ([32]byte, error) {
	return t.own.get(t.pl.g.Packages[q].RelPath, func() ([32]byte, error) {
		return t.pl.treeDigest(t.pl.own[q])
	})
}

// packageDigest is T(q), or the missing digest when no package sits at rel.
// "Directly nested" means q is the NEAREST enclosing package: enclosing lists
// are built nearest first, so that is element 0.
func (t *Traced) packageDigest(rel string) ([32]byte, error) {
	q, ok := t.pkgAt(rel)
	if !ok {
		return key.MissingPackageDigest(), nil
	}
	own, err := t.ownDigest(q)
	if err != nil {
		return [32]byte{}, fmt.Errorf("package %s: %w", short(rel), err)
	}
	var nested []string
	for _, j := range t.pl.nested[q] {
		if e := t.pl.enclosing[j]; len(e) > 0 && e[0] == q {
			nested = append(nested, t.pl.g.Packages[j].RelPath)
		}
	}
	sort.Strings(nested)
	return key.TracedPackageDigest(own, nested)
}

// pathDigest is the state of rel in the tracked tree: a file or link when
// Source lists it, a directory when it lists something beneath it, else absent.
func (t *Traced) pathDigest(rel string) ([32]byte, error) {
	pl := t.pl
	if pl.fileSet[rel] {
		info, err := pl.opt.Source.Describe(rel)
		if err != nil {
			return [32]byte{}, err
		}
		if info.Link {
			return key.LinkDigest(info.Target)
		}
		c, err := t.contentDigest(rel)
		if err != nil {
			return [32]byte{}, err
		}
		return key.FileDigest(info.Exec, c), nil
	}
	if len(pl.under(rel)) > 0 {
		return key.DirectoryDigest(), nil
	}
	return key.AbsentDigest(), nil
}

// contentDigest streams one tracked file through SHA-256.
func (t *Traced) contentDigest(rel string) ([32]byte, error) {
	rc, err := t.pl.opt.Source.Open(rel)
	if err != nil {
		return [32]byte{}, err
	}
	defer rc.Close()
	h := sha256.New()
	if _, err := io.Copy(h, rc); err != nil {
		return [32]byte{}, fmt.Errorf("%s: %w", short(rel), err)
	}
	var d [32]byte
	copy(d[:], h.Sum(nil))
	return d, nil
}

// listDigest is the listing of directory rel ("." is the root): the immediate
// children that hold or are tracked files, a child directory counting when
// anything tracked lies beneath it.  A path that is not a tracked directory
// (absent, a file or a link) gets the tagged not-a-directory digest, which no
// directory listing can equal.
func (t *Traced) listDigest(rel string) ([32]byte, error) {
	pl := t.pl
	var below []string
	if rel == "." {
		below = pl.files
	} else {
		below = pl.under(rel)
		if len(below) == 1 && below[0] == rel {
			below = nil // rel is itself a file or link
		}
	}
	prefix := ""
	if rel != "." {
		prefix = rel + "/"
	}
	kinds := map[string]uint8{}
	for _, f := range below {
		child, _, nested := strings.Cut(f[len(prefix):], "/")
		if nested {
			kinds[child] = key.ListDir
			continue
		}
		info, err := pl.opt.Source.Describe(f)
		if err != nil {
			return [32]byte{}, err
		}
		kinds[child] = key.ListFile
		if info.Link {
			kinds[child] = key.ListLink
		}
	}
	if len(kinds) == 0 {
		// Not a directory in the tracked tree (absent, a file or a link): a
		// tagged digest of its own, so this entry needs no paired path: entry.
		return key.NotDirectoryListingDigest(), nil
	}
	es := make([]key.ListingEntry, 0, len(kinds))
	for n, k := range kinds {
		es = append(es, key.ListingEntry{Name: n, Kind: k})
	}
	sort.Slice(es, func(a, b int) bool { return es[a].Name < es[b].Name })
	return key.ListingDigest(es)
}

// IsRefusal reports whether err is a *Refusal and returns it.
func IsRefusal(err error) (*Refusal, bool) {
	var r *Refusal
	if errors.As(err, &r) {
		return r, true
	}
	return nil, false
}

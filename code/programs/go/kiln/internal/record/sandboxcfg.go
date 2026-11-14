package record

// Generating the sandbox configuration (KILN04 section 4.8, stage 4b-1,
// decision P9)
// =====================================================================
//
// `kiln sandbox-exec` reads one configuration file (internal/sandbox/config.go)
// and builds, for every traced step, a new root that holds ONLY what the file
// names. This file decides what the recorder names. The inputs and the outputs:
//
//	host-roots.json (tracked)  --image, tool-->  ro_roots   read-only host paths
//	                           --state-------->  state_roots writable, but empty
//	the record's own paths     ----------------> home, tmpdir, kiln_exe, masks
//	the shard's scrubbed envs  ----------------> env_by_action
//
// Each choice below closes a hole the design review found.
//
// 1. `$HOME` roots. host-roots.json names `$HOME/.cargo/bin`, `$HOME/.rustup`,
//    `$HOME/.dotnet` and `$HOME/.ghcup`. The sandbox allows a read-only root
//    strictly beneath `home` (S1) and binds it AFTER home's fresh tmpfs. That
//    makes the step's `$HOME` a throw-away directory that still holds the
//    toolchains:
//
//        $HOME            fresh tmpfs, empty (the step's HOME)
//        $HOME/.rustup    the host's, read-only       <- a direct child: a mount
//                                                        point, cannot be renamed
//        $HOME/.cargo     fresh tmpfs, then SEALED    <- the ANCHOR of .cargo/bin
//        $HOME/.cargo/bin the host's, read-only
//        $HOME/.cargo/registry  host, WRITABLE        <- a declared state root
//
//    A root that is a direct child of home is a mount point and needs nothing.
//    A deeper root has an ANCHOR (the first directory below home) that the
//    sandbox seals read-only as a whole, so a step cannot rename the anchor and
//    rebuild `.cargo/bin` itself. Everything cargo must WRITE beneath the anchor
//    therefore has to be a state root, and that list is closed (stateAnchors):
//    a tool root under an anchor not in the table is refused. Anchoring
//    `$HOME/.local/bin` would seal ALL of `$HOME/.local`, which is exactly the
//    over-broad seal the §4.8 note warns about.
//
// 2. State roots start EMPTY. The recorder restores no cache, so a state root
//    that holds anything (`unscrubbed:state-preloaded`) fails the shard: a step
//    that reads a pre-existing crate would record a read of the machine, not of
//    the commit.
//
// 3. A root beneath `home` must be a real directory or file. A symlink there is
//    recreated as an ordinary inode of home's tmpfs, which a step may remove and
//    replace (M1), so it is refused here rather than at the first step.
//
// 4. `home` may not lie in a place the step can write (`/tmp`, `/var/tmp`,
//    tmpdir, the checkout, a state root) when any root lies beneath it (M2):
//    the step could rename an ancestor of home and rebuild the root. Validate
//    enforces it; the recorder does not weaken it.
//
// 5. `masks` is the checkout's `.git`. The commit pin protects object ids, not
//    the object store: a BUILD that could write `.git` could rewrite a loose
//    object or `.git/config`. In the sandbox there is no `.git` at all.
//
// 6. Kiln's own Git calls read `$HOME/.gitconfig`, `$XDG_CONFIG_HOME` and the
//    PATH. None may be writable from the sandbox, so no state root (and not the
//    checkout or tmpdir) may overlap any of them.
//
// 7. Nothing the recorder keeps to itself is visible: no ro root may overlap the
//    record directory, RUNNER_TEMP, the two binaries, or the directories of the
//    GitHub command files (GITHUB_ENV, GITHUB_PATH, GITHUB_OUTPUT).
//
// The function does no I/O beyond `lstat` and a directory read, both injectable,
// so every refusal is tested with literal paths on every OS.

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// stateAnchors is the closed table of anchors beneath home the recorder will
// seal. The key is the anchor's name below home; the value is what the
// toolchain writes beneath it, as state roots (paths below the anchor).
//
// `.cargo` holds `bin` (the tool root, read-only) next to the registry index and
// downloads and the git checkouts of dependencies, which cargo writes while it
// builds. Its `config.toml`, `env` and `.package-cache` stay absent or
// read-only (§4.8). rustup, dotnet and ghcup are direct children of home and
// have no anchor, so they need no entry.
var stateAnchors = map[string][]string{
	".cargo": {"registry", "git"},
}

// ConfigInput is everything the generator needs.
type ConfigInput struct {
	// Root is the checkout, Home the runner's $HOME, Tmpdir the step's TMPDIR
	// and KilnExe the canonical path of this binary. All are clean absolute
	// POSIX paths.
	Root, Home, Tmpdir, KilnExe string
	// XDGConfigHome is $XDG_CONFIG_HOME, "" when unset.
	XDGConfigHome string
	// Roots is the tracked host-roots.json.
	Roots *legacy.HostRoots
	// EnvByAction is every action the shard runs and its scrubbed environment.
	EnvByAction map[string][]string
	// Private are the places neither an ro root nor a state root may overlap, in
	// either direction (decision 7): the record directory, RUNNER_TEMP, the
	// binaries Kiln runs from the sandbox's side, the directories of the GitHub
	// command files.
	Private []Region
	// PrivateState are places a STATE root (writable) may not overlap but an ro
	// root may: the image's own strace and git live under /usr, which is a
	// read-only root, and so do the directories of the recorder's PATH. A
	// writable root over any of them would let a step plant a program the
	// recorder runs.
	PrivateState []Region

	// Lstat and ReadDir default to os.Lstat and os.ReadDir; tests inject fakes.
	Lstat   func(string) (os.FileInfo, error)
	ReadDir func(string) ([]os.DirEntry, error)
}

// BuildSandboxConfig makes and validates the configuration.
func BuildSandboxConfig(in ConfigInput) (*sandbox.Config, error) {
	lstat, readDir := in.Lstat, in.ReadDir
	if lstat == nil {
		lstat = os.Lstat
	}
	if readDir == nil {
		readDir = os.ReadDir
	}
	if in.Roots == nil {
		return nil, fmt.Errorf("sandbox config: host-roots.json is required")
	}
	if in.XDGConfigHome != "" && !path.IsAbs(in.XDGConfigHome) {
		return nil, fmt.Errorf("sandbox config: XDG_CONFIG_HOME %q is not an absolute path, so the version-control tool would read a configuration relative to its working directory", printable(in.XDGConfigHome))
	}
	if in.Home == "" || !path.IsAbs(in.Home) {
		return nil, fmt.Errorf("sandbox config: HOME %q is not an absolute path", printable(in.Home))
	}

	var ro, declared []string
	for _, r := range in.Roots.Roots {
		switch r.Kind {
		case legacy.RootImage, legacy.RootTool:
			p, err := expandRoot(r.Path, in.Home)
			if err != nil {
				return nil, err
			}
			ro = append(ro, p)
		case legacy.RootState:
			p, err := expandRoot(r.Path, in.Home)
			if err != nil {
				return nil, err
			}
			declared = append(declared, p)
		}
	}
	sort.Strings(ro)

	// Keep the roots that exist, and apply the rules for those beneath home.
	var kept []string
	anchors := map[string]bool{}
	for _, p := range ro {
		st, err := lstat(p)
		if err != nil {
			if os.IsNotExist(err) {
				continue // a runner without this toolchain: the language reads as failed
			}
			return nil, fmt.Errorf("sandbox config: ro root %q: %v", printable(p), err)
		}
		if anchor, beneath := sandbox.HomeAnchor(in.Home, p); beneath {
			if st.Mode()&os.ModeSymlink != 0 {
				return nil, fmt.Errorf("sandbox config: ro root %q beneath home is a symlink; a recreated symlink is not a mount point, list its target directory instead", printable(p))
			}
			if anchor != p {
				name := anchor[len(in.Home)+1:]
				if _, ok := stateAnchors[name]; !ok {
					return nil, fmt.Errorf("sandbox config: ro root %q would anchor (seal read-only) all of %q, which holds state the recorder does not declare; "+
						"add the anchor to the recorder's table with its state roots, or list a direct child of home", printable(p), printable(anchor))
				}
				anchors[anchor] = true
			}
		}
		kept = append(kept, p)
	}

	// State roots: what each used anchor's toolchain writes, then the declared
	// `state` roots of host-roots.json.
	var state []string
	for a := range anchors {
		for _, sub := range stateAnchors[a[len(in.Home)+1:]] {
			state = append(state, path.Join(a, sub))
		}
	}
	state = append(state, declared...)
	sort.Strings(state)
	state = dedupe(state)
	for _, s := range state {
		if err := stateRootEmpty(s, lstat, readDir); err != nil {
			return nil, err
		}
	}

	cfg := &sandbox.Config{
		RepoRoot:    in.Root,
		Home:        in.Home,
		Tmpdir:      in.Tmpdir,
		RoRoots:     nonNil(kept),
		StateRoots:  nonNil(state),
		Masks:       []string{path.Join(in.Root, ".git")},
		EnvByAction: in.EnvByAction,
		KilnExe:     in.KilnExe,
	}
	if cfg.EnvByAction == nil {
		cfg.EnvByAction = map[string][]string{}
	}
	// The sandbox's own rules first: they are the authority, and their
	// messages name the rule (M2, S1) that was broken.
	if err := cfg.Validate(); err != nil {
		return nil, err
	}
	if err := CheckRoRoots(cfg.RoRoots, in.Private); err != nil {
		return nil, err
	}
	for _, st := range cfg.StateRoots {
		for _, p := range append(append([]Region{}, in.Private...), in.PrivateState...) {
			if p.Path != "" && overlap(st, p.Path) {
				return nil, fmt.Errorf("sandbox config: state root %q overlaps %s %q, which a step must not be able to write", printable(st), p.Name, printable(p.Path))
			}
		}
	}
	if err := checkGitConfigUnwritable(cfg, in.XDGConfigHome); err != nil {
		return nil, err
	}
	return cfg, nil
}

// MarshalConfig renders the configuration as the one canonical byte string the
// binding hashes: compact JSON, no HTML escaping, map keys sorted by Go's
// encoder, every list non-null.
func MarshalConfig(c *sandbox.Config) ([]byte, error) {
	cc := *c
	cc.RoRoots, cc.StateRoots, cc.Masks = nonNil(cc.RoRoots), nonNil(cc.StateRoots), nonNil(cc.Masks)
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(&cc); err != nil {
		return nil, err
	}
	return bytes.TrimSuffix(buf.Bytes(), []byte("\n")), nil
}

// expandRoot replaces a leading $HOME. A root under $TMPDIR is refused: TMPDIR
// is a fresh tmpfs, so nothing of the host can be bound beneath it.
func expandRoot(p, home string) (string, error) {
	switch {
	case p == "$HOME":
		return home, nil
	case strings.HasPrefix(p, "$HOME/"):
		return path.Join(home, p[len("$HOME/"):]), nil
	case strings.HasPrefix(p, "$TMPDIR"):
		return "", fmt.Errorf("sandbox config: host root %q lies under $TMPDIR, which is a fresh tmpfs in the sandbox", printable(p))
	}
	return p, nil
}

// stateRootEmpty requires a state root to be absent, or an empty real directory.
func stateRootEmpty(p string, lstat func(string) (os.FileInfo, error), readDir func(string) ([]os.DirEntry, error)) error {
	st, err := lstat(p)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return fmt.Errorf("sandbox config: state root %q: %v", printable(p), err)
	}
	if st.Mode()&os.ModeSymlink != 0 || !st.IsDir() {
		return fmt.Errorf("sandbox config: state root %q is not a real directory", printable(p))
	}
	entries, err := readDir(p)
	if err != nil {
		return fmt.Errorf("sandbox config: state root %q: %v", printable(p), err)
	}
	if len(entries) != 0 {
		return fmt.Errorf("sandbox config: unscrubbed:state-preloaded: state root %q holds %d entries, but a recording restores no cache", printable(p), len(entries))
	}
	return nil
}

// checkGitConfigUnwritable refuses a writable place (a state root, the checkout,
// tmpdir) that overlaps a file or directory Kiln's own Git calls read:
// $HOME/.gitconfig, $HOME/.config/git, $XDG_CONFIG_HOME, and every directory of
// the PATHs the actions run with.
func checkGitConfigUnwritable(c *sandbox.Config, xdg string) error {
	reads := []string{path.Join(c.Home, ".gitconfig"), path.Join(c.Home, ".config", "git")}
	if xdg != "" {
		reads = append(reads, path.Clean(xdg))
	}
	seen := map[string]bool{}
	for _, env := range c.EnvByAction {
		for _, e := range env {
			list, ok := strings.CutPrefix(e, "PATH=")
			if !ok {
				continue
			}
			for _, d := range strings.Split(list, ":") {
				if path.IsAbs(d) && !seen[d] {
					seen[d] = true
					reads = append(reads, d)
				}
			}
		}
	}
	sort.Strings(reads)
	writable := append([]string{c.RepoRoot, c.Tmpdir}, c.StateRoots...)
	for _, r := range reads {
		for _, w := range writable {
			if overlap(r, w) {
				return fmt.Errorf("sandbox config: %q (read by Kiln's Git calls or the recorder's PATH) overlaps the writable place %q", printable(r), printable(w))
			}
		}
	}
	return nil
}

func nonNil(l []string) []string {
	if l == nil {
		return []string{}
	}
	return l
}

func dedupe(sorted []string) []string {
	out := sorted[:0:0]
	for i, s := range sorted {
		if i == 0 || s != sorted[i-1] {
			out = append(out, s)
		}
	}
	return out
}

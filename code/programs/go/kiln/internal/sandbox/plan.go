package sandbox

import (
	"fmt"
	"path"
	"sort"
)

// Linux mount flags, spelled here (rather than taken from package syscall) so
// the plan is pure and portable.  The values are the kernel's.
const (
	MsRdonly  = 0x1
	MsNosuid  = 0x2
	MsNodev   = 0x4
	MsNoexec  = 0x8
	MsRemount = 0x20
	MsBind    = 0x1000
	MsRec     = 0x4000
	MsPrivate = 0x40000
)

// MountKind says what a plan step does.
type MountKind string

const (
	// KindMakePrivate makes every mount of the new mount namespace private, so
	// nothing below propagates to the host (MS_REC|MS_PRIVATE on "/").
	KindMakePrivate MountKind = "make-private"
	// KindHostMkdir creates a state root (and its parents) on the host, in the
	// old root, before anything is mounted.
	KindHostMkdir MountKind = "host-mkdir"
	// KindStageTmpfs mounts the staging tmpfs on the host's TMPDIR, the one
	// place the new root is assembled.
	KindStageTmpfs MountKind = "stage-tmpfs"
	// KindStageDir creates the directory the new root is built on.
	KindStageDir MountKind = "stage-dir"
	// KindRootTmpfs mounts the new root itself: an EMPTY tmpfs.  The new root is
	// an allowlist, not a copy of "/": a pathname Unix socket or FIFO anywhere
	// in the host's tree is reachable by connect() or open() whatever the mount
	// flags, so nothing is visible that was not named.
	KindRootTmpfs MountKind = "root-tmpfs"
	// KindRORoot binds one allowed host root (a tool or image root from
	// host-roots.json) read-only into the new root: a directory or file is
	// bound, non-recursively (so no mount beneath it is carried along), and a
	// symlink such as /bin is recreated.
	KindRORoot MountKind = "ro-root"
	// KindTmpfs mounts a fresh tmpfs inside the new root.
	KindTmpfs MountKind = "tmpfs"
	// KindEnsureDir creates a mount point (and its parents) inside the new
	// root, where it is writable because an ancestor is a tmpfs of ours.
	KindEnsureDir MountKind = "ensure-dir"
	// KindBind binds a host path, writable, over a path of the new root.
	KindBind MountKind = "bind"
	// KindMask makes a path of the checkout absent: an empty read-only tmpfs
	// over a directory, an empty file over anything else.
	KindMask MountKind = "mask"
	// KindDev replaces /dev with a minimal tmpfs: the DevNodes bound from the
	// host's, the DevLinks, nothing else (no /dev/kvm, /dev/fuse, /dev/pts).
	KindDev MountKind = "dev"
	// KindProc mounts a fresh /proc for the new PID namespace.
	KindProc MountKind = "proc"
	// KindSeal makes the new root read-only, once everything is in place.
	KindSeal MountKind = "seal"
	// KindPivot makes the new root the root and detaches the old one.
	KindPivot MountKind = "pivot"
	// KindVerifyAbsent checks, after the pivot, that none of HostIPCPaths exists.
	KindVerifyAbsent MountKind = "verify-absent"
)

// DevNodes are the only device nodes the step gets, bound from the host's /dev.
var DevNodes = []string{"null", "zero", "full", "random", "urandom", "tty"}

// DevLink is a symlink of the minimal /dev.
type DevLink struct{ Name, Target string }

// DevLinks are the minimal /dev's symlinks.
var DevLinks = []DevLink{
	{"fd", "/proc/self/fd"}, {"stdin", "/proc/self/fd/0"}, {"stdout", "/proc/self/fd/1"}, {"stderr", "/proc/self/fd/2"},
}

// ScratchDirs are fresh, empty tmpfs in every sandbox besides $HOME and TMPDIR:
// the places a program expects to write, and where the host's service sockets
// and diagnostic sockets live.
var ScratchDirs = []string{"/tmp", "/var/tmp", "/run"}

// HostIPCPaths must not exist in the sandbox once it is built.  The helper
// refuses to run the step (exit 125) if any does: a read-only root that came
// to include one (say /var as a root, with its /var/run) must not be run.
var HostIPCPaths = []string{"/run/docker.sock", "/var/run/docker.sock", "/run/user", "/run/dbus", "/var/run/dbus"}

// replaced are the places the sandbox builds itself: no configured path may lie
// inside them.
var replaced = []string{"/dev", "/run", "/var/run", "/proc"}

// scratchRoots are the places that are fresh tmpfs: a read-only root inside one
// would be hidden by it.
var scratchRoots = []string{"/tmp", "/var/tmp"}

// Mount is one step of the plan.  Target is where it acts: for KindMakePrivate,
// KindHostMkdir and the stage kinds, a path of the OLD root; for KindRORoot and
// KindBind, Source is a path of the old root; every other Target is a path as
// the step will see it, to be placed under the new root.
type Mount struct {
	Kind   MountKind
	Source string
	Target string
	FSType string
	Flags  uintptr
	Data   string
}

// StageDirName is the directory, under TMPDIR, the new root is built on.
const StageDirName = ".kiln-newroot"

// StageRoot is the path, in the old root, where the new root is built.
func StageRoot(c *Config) string { return path.Join(c.Tmpdir, StageDirName) }

// tmpfsData is the option string of one of our scratch tmpfs mounts.  The
// owner is the mapped id so the step can write; the mode keeps others out of
// $HOME and lets TMPDIR be shared like a normal /tmp.
func tmpfsData(mode string) string {
	return fmt.Sprintf("mode=%s,uid=%d,gid=%d", mode, InsideID, InsideID)
}

// devData bounds the minimal /dev and makes it the step's.
var devData = fmt.Sprintf("mode=0755,size=64k,nr_inodes=64,uid=%d,gid=%d", InsideID, InsideID)

// maskData bounds a mask's tmpfs to one page and one inode: it only has to be
// empty.
const maskData = "mode=0555,size=4k,nr_inodes=2"

// Plan returns the ordered mount steps for a configuration.  It is pure: the
// order IS the specification (KILN04 section 4.7), because each step depends on
// the ones before it.
//
//  1. Make every mount private.
//  2. Create the state roots on the host.
//  3. Mount the staging tmpfs on TMPDIR, create the new root's directory in it,
//     and mount the new root, an empty tmpfs, there.
//  4. Bind each allowed read-only root into it (shallowest first), creating the
//     missing parent directories on the root tmpfs.
//  5. Fresh tmpfs for HOME, TMPDIR, /tmp, /var/tmp and /run, shallowest first.
//  6. The minimal /dev.
//  7. Bind the repository, writable, at its own path, and then each state root.
//  8. Mask the checkout's own files (.git and the like), which are direct
//     children of it.
//  9. A fresh /proc with hidepid=2.
//  10. Seal the root read-only, pivot into it, verify no host IPC path exists.
//
// The scratch tmpfs come before the checkout and the state roots because the
// checkout may live under $HOME (or /tmp): the missing parents are created in
// the tmpfs, and a bind made earlier would be hidden by it.
func Plan(c *Config) ([]Mount, error) {
	if err := c.Validate(); err != nil {
		return nil, err
	}
	stage := StageRoot(c)
	plan := []Mount{{Kind: KindMakePrivate, Target: "/", Flags: MsRec | MsPrivate}}
	for _, s := range c.StateRoots {
		plan = append(plan, Mount{Kind: KindHostMkdir, Target: s})
	}
	plan = append(plan,
		Mount{Kind: KindStageTmpfs, Target: c.Tmpdir, FSType: "tmpfs", Flags: MsNosuid | MsNodev, Data: tmpfsData("0755")},
		Mount{Kind: KindStageDir, Target: stage},
		Mount{Kind: KindRootTmpfs, Target: stage, FSType: "tmpfs", Flags: MsNosuid | MsNodev, Data: tmpfsData("0755")},
	)
	for _, r := range sortedByDepth(c.RoRoots) {
		plan = append(plan, Mount{Kind: KindRORoot, Source: r, Target: r, Flags: MsBind})
	}
	// The scratch tmpfs, deduplicated (TMPDIR may be /tmp) and shallowest first
	// so a parent is mounted before what is created inside it.
	mode := map[string]string{c.Home: "0755", c.Tmpdir: "1777"}
	for _, d := range ScratchDirs {
		if _, ok := mode[d]; !ok {
			mode[d] = "1777"
			if d == "/run" {
				mode[d] = "0755"
			}
		}
	}
	scratch := make([]string, 0, len(mode))
	for p := range mode {
		scratch = append(scratch, p)
	}
	for _, p := range sortedByDepth(scratch) {
		plan = append(plan,
			Mount{Kind: KindEnsureDir, Target: p},
			Mount{Kind: KindTmpfs, Target: p, FSType: "tmpfs", Flags: MsNosuid | MsNodev, Data: tmpfsData(mode[p])},
		)
	}
	plan = append(plan,
		Mount{Kind: KindEnsureDir, Target: "/dev"},
		Mount{Kind: KindDev, Target: "/dev", FSType: "tmpfs", Flags: MsNosuid | MsNoexec, Data: devData},
		Mount{Kind: KindEnsureDir, Target: c.RepoRoot},
		Mount{Kind: KindBind, Source: c.RepoRoot, Target: c.RepoRoot, Flags: MsBind},
	)
	for _, s := range c.StateRoots {
		plan = append(plan,
			Mount{Kind: KindEnsureDir, Target: s},
			Mount{Kind: KindBind, Source: s, Target: s, Flags: MsBind},
		)
	}
	for _, m := range sortedByDepth(c.Masks) {
		plan = append(plan, Mount{Kind: KindMask, Target: m, FSType: "tmpfs", Flags: MsRdonly | MsNosuid | MsNodev | MsNoexec, Data: maskData})
	}
	plan = append(plan,
		Mount{Kind: KindEnsureDir, Target: "/proc"},
		Mount{Kind: KindProc, Source: "proc", Target: "/proc", FSType: "proc", Flags: MsNosuid | MsNodev | MsNoexec, Data: "hidepid=2"},
		Mount{Kind: KindSeal, Target: stage, Flags: MsRemount | MsRdonly | MsNosuid | MsNodev},
		Mount{Kind: KindPivot, Target: stage},
		Mount{Kind: KindVerifyAbsent},
	)
	return plan, nil
}

// sortedByDepth returns a copy of paths ordered by depth, then name.
func sortedByDepth(paths []string) []string {
	out := append([]string{}, paths...)
	sort.Slice(out, func(i, j int) bool {
		if di, dj := depth(out[i]), depth(out[j]); di != dj {
			return di < dj
		}
		return out[i] < out[j]
	})
	return out
}

// depth counts a clean absolute path's components.
func depth(p string) int {
	n := 0
	for _, r := range p {
		if r == '/' {
			n++
		}
	}
	return n
}

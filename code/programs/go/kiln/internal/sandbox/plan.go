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
	// old root, before the filesystem is made read-only.
	KindHostMkdir MountKind = "host-mkdir"
	// KindStageTmpfs mounts the scratch tmpfs on the host's TMPDIR, the one
	// place the new root is assembled.
	KindStageTmpfs MountKind = "stage-tmpfs"
	// KindStageDir creates the directory the new root is built in.
	KindStageDir MountKind = "stage-dir"
	// KindRootBind makes the new root: a recursive bind of "/".
	KindRootBind MountKind = "root-bind"
	// KindReadOnly makes the new root, and every mount beneath it, read-only
	// (mount_setattr with MOUNT_ATTR_RDONLY and AT_RECURSIVE).
	KindReadOnly MountKind = "read-only"
	// KindTmpfs mounts a fresh tmpfs inside the new root.
	KindTmpfs MountKind = "tmpfs"
	// KindEnsureDir creates a mount point inside the new root, where it is
	// writable (an ancestor is a tmpfs of ours).
	KindEnsureDir MountKind = "ensure-dir"
	// KindBind binds a host path, writable, over a path of the new root.
	KindBind MountKind = "bind"
	// KindMask makes a path of the new root absent: an empty read-only tmpfs
	// over a directory, an empty file over anything else.  Which one is
	// decided when the target is looked at.
	KindMask MountKind = "mask"
	// KindProc mounts a fresh /proc for the new PID namespace.
	KindProc MountKind = "proc"
	// KindPivot makes the new root the root and detaches the old one.
	KindPivot MountKind = "pivot"
)

// Mount is one step of the plan.  Target is where it acts: for the "stage"
// kinds and KindMakePrivate, KindHostMkdir and KindRootBind's source, a path
// of the OLD root; otherwise a path as the step will see it, to be placed
// under the new root.
type Mount struct {
	Kind   MountKind
	Source string
	Target string
	FSType string
	Flags  uintptr
	Data   string
	// InRepo marks a KindMask whose target lies inside the checkout, where a
	// missing target is a refusal (the step could create it).
	InRepo bool
}

// StageDirName is the directory, under TMPDIR, the new root is built in.
const StageDirName = ".kiln-newroot"

// StageRoot is the path, in the old root, where the new root is built.
func StageRoot(c *Config) string { return path.Join(c.Tmpdir, StageDirName) }

// tmpfsData is the option string of one of our scratch tmpfs mounts.  The
// owner is the mapped id so the step can write; the mode keeps others out of
// $HOME and lets TMPDIR be shared like a normal /tmp.
func tmpfsData(mode string) string {
	return fmt.Sprintf("mode=%s,uid=%d,gid=%d", mode, InsideID, InsideID)
}

// maskData bounds a mask's tmpfs to one page and one inode: it only has to be
// empty.
const maskData = "mode=0555,size=4k,nr_inodes=2"

// Plan returns the ordered mount steps for a configuration.  It is pure: the
// order IS the specification (KILN04 section 4.7), because each step depends on
// the ones before it.
//
//  1. Make every mount private.
//  2. Create the state roots on the host.
//  3. Mount the staging tmpfs on TMPDIR and create the new root's directory.
//  4. Recursively bind "/" onto it and make that whole tree read-only.
//  5. Mount fresh tmpfs over HOME and TMPDIR (the shallower first).
//  6. Bind the repository, writable, at its own path.
//  7. Bind each state root, writable.
//  8. Mask: .git, the trace and ledger directories, and the rest, shallowest
//     first.
//  9. Mount a fresh /proc with hidepid=2.
//  10. Pivot into the new root.
//
// Home and TMPDIR come before the repository and the state roots because the
// checkout may live under $HOME: the tmpfs would hide a bind made earlier.
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
		Mount{Kind: KindRootBind, Source: "/", Target: stage, Flags: MsBind | MsRec},
		Mount{Kind: KindReadOnly, Target: stage},
	)
	scratch := []struct{ p, mode string }{{c.Home, "0755"}, {c.Tmpdir, "1777"}}
	if within(c.Home, c.Tmpdir) {
		scratch[0], scratch[1] = scratch[1], scratch[0] // mount the parent first
	}
	for _, s := range scratch {
		plan = append(plan, Mount{Kind: KindEnsureDir, Target: s.p})
		plan = append(plan, Mount{Kind: KindTmpfs, Target: s.p, FSType: "tmpfs", Flags: MsNosuid | MsNodev, Data: tmpfsData(s.mode)})
	}
	plan = append(plan,
		Mount{Kind: KindEnsureDir, Target: c.RepoRoot},
		Mount{Kind: KindBind, Source: c.RepoRoot, Target: c.RepoRoot, Flags: MsBind},
	)
	for _, s := range c.StateRoots {
		plan = append(plan,
			Mount{Kind: KindEnsureDir, Target: s},
			Mount{Kind: KindBind, Source: s, Target: s, Flags: MsBind},
		)
	}
	masks := append([]string{}, c.Masks...)
	sort.Slice(masks, func(i, j int) bool {
		if di, dj := depth(masks[i]), depth(masks[j]); di != dj {
			return di < dj
		}
		return masks[i] < masks[j]
	})
	for _, m := range masks {
		plan = append(plan, Mount{Kind: KindMask, Target: m, FSType: "tmpfs", Flags: MsRdonly | MsNosuid | MsNodev | MsNoexec, Data: maskData, InRepo: within(m, c.RepoRoot)})
	}
	plan = append(plan,
		Mount{Kind: KindProc, Source: "proc", Target: "/proc", FSType: "proc", Flags: MsNosuid | MsNodev | MsNoexec, Data: "hidepid=2"},
		Mount{Kind: KindPivot, Target: stage},
	)
	return plan, nil
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

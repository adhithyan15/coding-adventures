package sandbox

// The privilege drop (KILN04 section 4.3 step 4, section 4.7), described as
// data so its order can be tested on any OS.  The Linux executor interprets
// each step with one system call.

// PrivOp names one step of the drop.
type PrivOp string

const (
	// PrivNoNewPrivs sets PR_SET_NO_NEW_PRIVS: no exec gains privilege (setuid
	// files, file capabilities), and an unprivileged seccomp filter may be
	// installed.
	PrivNoNewPrivs PrivOp = "no-new-privs"
	// PrivBoundingDrop drops capabilities 0..Arg-1 from the bounding set with
	// PR_CAPBSET_DROP.  EINVAL means "past the last capability this kernel
	// has" and ends the loop.  It needs CAP_SETPCAP, so it comes before the
	// capset that removes it.
	PrivBoundingDrop PrivOp = "bounding-drop"
	// PrivAmbientClear empties the ambient set (PR_CAP_AMBIENT_CLEAR_ALL).
	PrivAmbientClear PrivOp = "ambient-clear"
	// PrivCapsetEmpty sets the effective, permitted and inheritable sets to
	// empty with capset (version 3).
	PrivCapsetEmpty PrivOp = "capset-empty"
)

// PrivStep is one step: an operation and its argument.
type PrivStep struct {
	Op  PrivOp
	Arg uint64
}

// MaxCapability is the exclusive upper bound of the bounding-set loop.  The
// kernel's last capability is below 64 today; the loop ends at the first
// EINVAL, so a larger number costs nothing and a future capability is covered.
const MaxCapability = 64

// PrivSteps returns the drop sequence, in the only order that works:
// capabilities are used (SETPCAP for the bounding set) until the last step
// removes them.
func PrivSteps() []PrivStep {
	return []PrivStep{
		{PrivNoNewPrivs, 1},
		{PrivBoundingDrop, MaxCapability},
		{PrivAmbientClear, 0},
		{PrivCapsetEmpty, 0},
	}
}

// Capabilities the init is started with (ambient), the minimum its setup uses.
// They are gone after PrivSteps.
const (
	CapSetpcap  = 8  // PR_CAPBSET_DROP and changing inheritable sets
	CapSysAdmin = 21 // mount, pivot_root, unshare(CLONE_FS), writing id maps
)

// StartCaps is the ambient set the root asks the kernel to give the init, so
// the capabilities of its new user namespace survive the exec of a non-root
// uid.
func StartCaps() []uintptr { return []uintptr{CapSetpcap, CapSysAdmin} }

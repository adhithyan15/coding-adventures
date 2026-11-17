// Package policyfix is test support: it hands out the `ephemeral` and
// `absent-ok` lists of the REAL code/kiln/legacy/host-roots.json, so every
// fixture repository that needs a host-roots.json of its own (with its own
// `roots`) carries exactly the committed policy for those two lists and cannot
// drift from it (KILN04 D27).
//
// Why it matters: ubuntu-26.04's libselinux-linked tools read /proc/filesystems
// and statfs /sys/fs/selinux and /selinux at start-up.  A fixture with a short
// hand-written list would fail on that image although the real policy allows
// it; a fixture that was MORE permissive than the real policy would hide a
// regression.  Deriving the lists removes both errors.
//
// Only tests import this package.
package policyfix

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
)

// Lists returns the real policy's ephemeral and absent-ok lists as JSON array
// text, ready to splice into a host-roots.json.  The file is read with the
// production parser (strict ordering and shape rules included).  It panics on
// any problem: a test fixture cannot continue without the real policy.
func Lists() (ephemeral, absentOK string) {
	_, here, _, ok := runtime.Caller(0)
	if !ok {
		panic("policyfix: cannot locate the source tree")
	}
	// here = <repo>/code/programs/go/kiln/internal/policyfix/policyfix.go
	dir := filepath.Join(filepath.Dir(here), "..", "..", "..", "..", "..", "kiln", "legacy")
	tcData, err := os.ReadFile(filepath.Join(dir, "toolchains.json"))
	if err != nil {
		panic(fmt.Sprintf("policyfix: %v", err))
	}
	tc, err := legacy.ParseToolchains(tcData)
	if err != nil {
		panic(fmt.Sprintf("policyfix: %v", err))
	}
	data, err := os.ReadFile(filepath.Join(dir, "host-roots.json"))
	if err != nil {
		panic(fmt.Sprintf("policyfix: %v", err))
	}
	hr, err := legacy.ParseHostRoots(data, tc)
	if err != nil {
		panic(fmt.Sprintf("policyfix: %v", err))
	}
	e, _ := json.Marshal(hr.Ephemeral)
	a, _ := json.Marshal(hr.AbsentOK)
	return string(e), string(a)
}

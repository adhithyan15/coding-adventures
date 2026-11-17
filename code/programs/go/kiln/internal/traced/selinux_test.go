package traced

// KILN04 decision D27: libselinux (coreutils, grep, sed on ubuntu-26.04) probes
// /sys/fs/selinux and /selinux with statfs and opens /proc/filesystems at
// start-up.  These tests pin how the REAL host-roots.json treats those probes:
//
//	/proc/filesystems          ephemeral: constant per kernel build, and the
//	                           image id is already in the weak key.
//	/selinux, /sys/fs/selinux  absent-ok: allowed ONLY while absent.  When
//	                           SELinux is enabled the host has state that
//	                           changes outputs (ls -Z, cp --preserve=context),
//	                           so the probe finds the directory, which is a
//	                           host-read, and the action records nothing.
//
// The invariant: an SELinux-enabled host records nothing.

import (
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// realHostRoots parses code/kiln/legacy/host-roots.json with the production
// parser, so the ordering and shape rules are enforced too.
func realHostRoots(t *testing.T) *legacy.HostRoots {
	t.Helper()
	dir := filepath.Join("..", "..", "..", "..", "..", "kiln", "legacy")
	tcData, err := os.ReadFile(filepath.Join(dir, "toolchains.json"))
	if err != nil {
		t.Fatal(err)
	}
	tc, err := legacy.ParseToolchains(tcData)
	if err != nil {
		t.Fatal(err)
	}
	data, err := os.ReadFile(filepath.Join(dir, "host-roots.json"))
	if err != nil {
		t.Fatal(err)
	}
	hr, err := legacy.ParseHostRoots(data, tc)
	if err != nil {
		t.Fatal(err)
	}
	return hr
}

// withRealLists gives the harness the real ephemeral and absent-ok lists,
// unexpanded: traced's own prepare step expands $HOME and $TMPDIR, the same
// code path production uses (the classify file carries the lists as written),
// so there is no second expansion here to drift.  The harness roots stay.
func withRealLists(t *testing.T, h *harness) {
	t.Helper()
	hr := realHostRoots(t)
	h.cfg.HostRoots.Ephemeral = hr.Ephemeral
	h.cfg.HostRoots.AbsentOK = hr.AbsentOK
}

func statfs(path string) trace.Event { return base("statfs", trace.KindMetadata, path) }

func TestSELinuxProbesWithTheRealPolicy(t *testing.T) {
	tests := []struct {
		name  string
		event trace.Event
		want  string // "" means the record is kept
	}{
		{"proc filesystems read", openAbs("/proc/filesystems"), ""},
		{"absent statfs /sys/fs/selinux", fail(statfs("/sys/fs/selinux"), "ENOENT"), ""},
		{"absent statfs /selinux", fail(statfs("/selinux"), "ENOENT"), ""},
		{"present /sys/fs/selinux", statfs("/sys/fs/selinux"), "host-read:/sys/fs/selinux"},
		{"present /selinux", statfs("/selinux"), "host-read:/selinux"},
		{"absent child of an absent entry", fail(statfs("/sys/fs/selinux/enforce"), "ENOENT"), ""},
		{"selinux enforce", openAbs("/sys/fs/selinux/enforce"), "host-read:/sys/fs/selinux/enforce"},
		{"proc mounts", openAbs("/proc/mounts"), "trace-incomplete:proc-path"},
		{"proc self attr current", openAbs("/proc/self/attr/current"), "trace-incomplete:proc-path"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			h := newHarness(t)
			withRealLists(t, h)
			rec := h.run(tc.event)
			if tc.want == "" {
				wantNoReasons(t, rec)
			} else {
				wantReason(t, rec, tc.want)
			}
		})
	}
}

func TestRealHostRootsSELinuxMembership(t *testing.T) {
	hr := realHostRoots(t)
	has := func(l []string, s string) bool {
		for _, x := range l {
			if x == s {
				return true
			}
		}
		return false
	}
	if !has(hr.Ephemeral, "/proc/filesystems") {
		t.Error("/proc/filesystems must be ephemeral")
	}
	for _, p := range []string{"/selinux", "/sys/fs/selinux"} {
		if !has(hr.AbsentOK, p) {
			t.Errorf("%s must be absent-ok", p)
		}
		if has(hr.Ephemeral, p) {
			t.Errorf("%s must never be ephemeral: present means SELinux is on", p)
		}
	}
	// No ephemeral entry or root may cover anything under the SELinux trees.
	covers := func(entry, p string) bool {
		if strings.HasSuffix(entry, "/**") {
			b := strings.TrimSuffix(entry, "/**")
			return p == b || strings.HasPrefix(p, b+"/")
		}
		return entry == p || strings.HasPrefix(p, entry+"/")
	}
	for _, p := range []string{"/sys/fs/selinux/enforce", "/sys/fs/selinux/policy", "/selinux/enforce"} {
		for _, e := range hr.Ephemeral {
			if covers(e, p) {
				t.Errorf("ephemeral entry %q covers %s", e, p)
			}
		}
		for _, r := range hr.Roots {
			if covers(r.Path, p) {
				t.Errorf("root %q covers %s", r.Path, p)
			}
		}
	}
	if has(hr.AbsentOK, "/proc/mounts") || has(hr.Ephemeral, "/proc/mounts") {
		t.Error("/proc/mounts must stay trace-incomplete")
	}
}

// L1: a probe of a CHILD of an absent-ok entry is allowed only while the entry
// itself is absent on the classifier's disk (ENOENT or ENOTDIR); a present
// entry (even an empty directory) or any other answer is a host-read.
func TestAbsentOKChildNeedsAbsentEntry(t *testing.T) {
	child := func() trace.Event { return fail(stat("/sys/fs/selinux/enforce"), "ENOENT") }
	exact := func() trace.Event { return fail(stat("/sys/fs/selinux"), "ENOENT") }
	fresh := func() *harness {
		h := newHarness(t)
		withRealLists(t, h)
		return h
	}

	wantNoReasons(t, fresh().run(child()))

	// ENOTDIR from the disk (a file sits where the entry should be) is absent too.
	h := fresh()
	h.fs.errs["/sys/fs/selinux"] = syscall.ENOTDIR
	wantNoReasons(t, h.run(child()))

	// Present, empty directory: the failed child probe is not enough.
	h = fresh()
	h.fs.dir("/sys/fs/selinux")
	wantReason(t, h.run(child()), "host-read:/sys/fs/selinux/enforce")

	// The entry cannot be examined: fail closed.
	h = fresh()
	h.fs.errs["/sys/fs/selinux"] = syscall.EACCES
	wantReason(t, h.run(child()), "host-read:/sys/fs/selinux/enforce")

	// An exact probe uses the traced result alone, whatever the disk says.
	h = fresh()
	h.fs.errs["/sys/fs/selinux"] = syscall.EACCES
	wantNoReasons(t, h.run(exact()))

	// $HOME entries live in the fresh tmpfs: a child probe needs no disk answer.
	h = fresh()
	h.fs.dir("/home/runner/.config")
	wantNoReasons(t, h.run(fail(stat("/home/runner/.config/x"), "ENOENT")))
}

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

// withRealLists gives the harness the real ephemeral and absent-ok lists
// ($HOME expanded to the harness's home); the harness roots stay as they are.
func withRealLists(t *testing.T, h *harness) {
	t.Helper()
	hr := realHostRoots(t)
	exp := func(l []string) []string {
		out := make([]string, len(l))
		for i, s := range l {
			out[i] = strings.Replace(s, "$HOME", h.cfg.Home, 1)
		}
		return out
	}
	h.cfg.HostRoots.Ephemeral = exp(hr.Ephemeral)
	h.cfg.HostRoots.AbsentOK = exp(hr.AbsentOK)
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

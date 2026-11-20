//go:build linux && amd64

package main

// KILN04 D43, the sealed root, against the real sandbox.
//
// The classifier (internal/traced, sealed.go) treats a path as absent without
// asking the host's disk when the new root cannot contain it.  That is a claim
// about the sandbox, so this test checks it where it is made: inside a real
// sandbox, the places the census named (the resolver stub under /run, the
// go.work and package.json above the checkout, /home, /opt/rh, /var/run) are
// ENOENT even when the HOST has them, nothing can be created at them (the root
// is sealed read-only), and the places the classifier takes for fresh tmpfs
// (/run, /tmp, /var/tmp) are writable and empty.  The pure half, that the
// classifier's exists-set covers everything the plan mounts, is
// internal/record/sealedroot_test.go.

import (
	"os"
	"strings"
	"testing"
)

func TestSandboxSealedRoot(t *testing.T) {
	f := newSandboxFixture(t)
	// The host has a /home/x only if the machine has a /home; what matters is
	// that the step cannot see it.  A file at a path the sandbox must lack is
	// created for the host side of the check when we are allowed to.
	for _, p := range []string{"/go.work", "/home/x", "/opt/rh", "/run/x", "/var/run", "/package.json", "/run/systemd/resolve/stub-resolv.conf", "/var/run/docker.sock"} {
		if _, err := os.Lstat(p); err == nil {
			t.Logf("F2: the host has %s; the sandbox must not show it", p)
		}
	}
	script := strings.Join([]string{
		`for p in /go.work /home/x /opt/rh /run/x /var/run /package.json /run/systemd/resolve/stub-resolv.conf /var/run/docker.sock /include /pyvenv.cfg /home /opt /var /var/lib; do ` +
			`echo "ls:$p:$(ls -d $p 2>&1)"; done`,
		// Sealed read-only: nothing can be made above the mounts.
		`for p in /package.json /home /go.work /var/lib /opt; do if mkdir $p 2>/dev/null || touch $p 2>/dev/null; then echo "sealed-writable:$p"; else echo "sealed-readonly:$p"; fi; done`,
		// The scratch tmpfs are writable and start empty.
		// (/tmp also holds the directories the plan makes to mount the fixture beneath
		// it, so only /run and /var/tmp are counted.)
		`for d in /run /var/tmp; do echo "scratch:$d:$(ls -A $d | wc -l)"; done`,
		`for d in /run /tmp /var/tmp; do echo x > $d/own && echo "scratch-writable:$d"; done`,
		// The name the action made is there; the one it did not make still is not.
		// (An if, not `test && echo`: the expected case is that /run/systemd is
		// absent, and a false `test` as the last command would be the script's
		// exit status.)
		`if test -e /run/own; then echo run-own-present; fi`,
		`if test -e /run/systemd; then echo run-systemd-present; fi`,
	}, "; ")
	r := f.sandboxRun(t, "pkg", "sh", "-c", script)
	if r.code != 0 {
		t.Fatalf("exit %d: stderr %q\nstdout:\n%s", r.code, r.stderr, r.stdout)
	}
	out := r.stdout
	for _, p := range []string{"/go.work", "/home/x", "/opt/rh", "/run/x", "/var/run", "/package.json", "/run/systemd/resolve/stub-resolv.conf", "/var/run/docker.sock", "/include", "/pyvenv.cfg"} {
		want := "ls:" + p + ":"
		i := strings.Index(out, want)
		if i < 0 {
			t.Errorf("no result for %s:\n%s", p, out)
			continue
		}
		line := out[i:]
		if j := strings.IndexByte(line, '\n'); j >= 0 {
			line = line[:j]
		}
		if !strings.Contains(line, "No such file or directory") {
			t.Errorf("%s is not ENOENT in the sandbox: %q", p, line)
		}
	}
	for _, p := range []string{"/package.json", "/home", "/go.work", "/var/lib", "/opt"} {
		if !strings.Contains(out, "sealed-readonly:"+p+"\n") {
			t.Errorf("%s could be created in the sandbox (the new root is not sealed):\n%s", p, out)
		}
	}
	if strings.Contains(out, "sealed-writable:") {
		t.Errorf("something was created above the mounts:\n%s", out)
	}
	for _, d := range []string{"/run", "/tmp", "/var/tmp"} {
		wants := []string{"scratch-writable:" + d + "\n"}
		if d != "/tmp" {
			wants = append(wants, "scratch:"+d+":0\n")
		}
		for _, want := range wants {
			if !strings.Contains(out, want) {
				t.Errorf("output lacks %q:\n%s", want, out)
			}
		}
	}
	if !strings.Contains(out, "run-own-present") || strings.Contains(out, "run-systemd-present") {
		t.Errorf("/run is not the step's own fresh tmpfs:\n%s", out)
	}
}

package traced

import (
	"strings"
	"testing"
	"time"
)

func TestImageRoots(t *testing.T) {
	h := newHarness(t)
	h.image("/usr/lib/libm.so", "/etc/passwd")
	// An unmodified image file is allowed, and so is probing for a missing one.
	rec := h.run(openAbs("/usr/lib/libm.so"), stat("/etc/passwd"), fail(stat("/usr/lib/absent"), "ENOENT"))
	wantNoReasons(t, rec)
	eq(t, "entries", rec.Entries)

	// A file a setup step changed after the job started.
	h.fs.info["/usr/lib/libm.so"] = FileInfo{Kind: KindFile, Ctime: jobStart.Add(time.Second)}
	rec = h.run(openAbs("/usr/lib/libm.so"))
	eq(t, "reasons", rec.Reasons, "host-read:setup-modified:/usr/lib/libm.so")

	// Exactly at the stamp is old enough.
	h.fs.info["/usr/lib/libm.so"] = FileInfo{Kind: KindFile, Ctime: jobStart}
	wantNoReasons(t, h.run(openAbs("/usr/lib/libm.so")))

	// A zero ctime, or a file the disk lost, cannot be shown to be old.
	h.fs.info["/usr/lib/libm.so"] = FileInfo{Kind: KindFile}
	wantReason(t, h.run(openAbs("/usr/lib/libm.so")), "host-read:/usr/lib/libm.so")
	wantReason(t, h.run(openAbs("/usr/lib/gone")), "host-read:/usr/lib/gone")
}

func TestToolRoots(t *testing.T) {
	h := newHarness(t)
	// go is in the weak key's tools; ruby is not.
	rec := h.run(openAbs("/opt/tc/go/1.26/bin/go"), fail(stat("/opt/tc/go/1.26/x"), "ENOENT"))
	wantNoReasons(t, rec)
	rec = h.run(openAbs("/opt/tc/ruby/3.4/bin/ruby"), fail(stat("/opt/tc/ruby/3.4/x"), "ENOENT"))
	eq(t, "reasons", rec.Reasons, "host-read:tool-not-keyed:ruby")
	// Another version of a tool is outside its root.
	wantReason(t, h.run(openAbs("/opt/tc/go/1.27/bin/go")), "host-read:/opt/tc/go/1.27/bin/go")
}

func TestStateRoots(t *testing.T) {
	h := newHarness(t)
	// Until fetchers exist, every state read is a host-read; an absent probe is
	// fine because state roots start empty.
	wantReason(t, h.run(openAbs("/home/runner/.cargo/registry/index")), "host-read:$HOME/.cargo/registry/index")
	wantNoReasons(t, h.run(fail(stat("/home/runner/.cargo/registry/x"), "ENOENT")))
	// A writer under a state root is a non-fetcher.
	wantReason(t, h.run(open("/home/runner/.cargo/registry/x", "O_WRONLY|O_CREAT|O_EXCL", "/home/runner/.cargo/registry/x")),
		"state-write:$HOME/.cargo/registry/x")

	// With fetchers on, a pinned ecosystem's state may be read; an unpinned one not.
	old := fetchersEnabled
	fetchersEnabled = true
	defer func() { fetchersEnabled = old }()
	wantNoReasons(t, h.run(openAbs("/home/runner/.cargo/registry/index")))
	h.cfg.Pinned = map[string]bool{}
	wantReason(t, h.run(openAbs("/home/runner/.cargo/registry/index")), "host-read:$HOME/.cargo/registry/index")
}

func TestEphemeralAndAbsentOK(t *testing.T) {
	h := newHarness(t)
	rec := h.run(openAbs("/dev/null"), stat("/sys/devices/system/cpu/online"), stat("/sys/devices/system/cpu"),
		openAbs("/usr/special/anything"))
	wantNoReasons(t, rec)
	// absent-ok: a dotfile probe is fine, reading one is not.
	wantNoReasons(t, h.run(fail(stat("/home/runner/.npmrc"), "ENOENT"), fail(stat("/home/runner/.config/pip/pip.conf"), "ENOENT")))
	rec = h.run(openAbs("/home/runner/.config/x"))
	eq(t, "reasons", rec.Reasons, "host-read:$HOME/.config/x")
	// Other absent probes are host-reads too.
	wantReason(t, h.run(fail(stat("/home/runner/.gitconfig"), "ENOENT")), "host-read:$HOME/.gitconfig")
	wantReason(t, h.run(fail(stat("/var/nothing"), "ENOENT")), "host-read:/var/nothing")
}

func TestOwnTemporaryFilesAreAllowed(t *testing.T) {
	h := newHarness(t)
	rec := h.run(
		mkdir("/tmp/kiln-1/d"),
		open("/tmp/kiln-1/d/f", "O_WRONLY|O_CREAT|O_EXCL", "/tmp/kiln-1/d/f"),
		openAbs("/tmp/kiln-1/d/f"),
		fail(stat("/tmp/other"), "ENOENT"),
	)
	// Only the probe of someone else's path is a host-read.
	eq(t, "reasons", rec.Reasons, "host-read:/tmp/other")
	// A temp file the action did not make is a host-read, with TMPDIR redacted.
	wantReason(t, h.run(openAbs("/tmp/kiln-1/stale")), "host-read:$TMPDIR/stale")
}

func TestReasonsRedactAndSanitize(t *testing.T) {
	h := newHarness(t)
	rec := h.run(openAbs("/home/runner/secret/key"), openAbs("/srv/a\nb\x01"), openAbs("/srv/"+strings.Repeat("x", 400)))
	for _, r := range rec.Reasons {
		if strings.ContainsAny(r, "\n\x01") || strings.Contains(r, "/home/runner") || len(r) > 260 {
			t.Errorf("reason %q is not safe to print", r)
		}
	}
	wantReason(t, rec, "host-read:$HOME/secret/key")
	// The longer of two overlapping prefixes wins.
	h.cfg.Tmpdir = "/home/runner/tmp"
	wantReason(t, h.run(openAbs("/home/runner/tmp/q")), "host-read:$TMPDIR/q")
	// Reads inside the repository print their own relative path, unredacted.
	h = newHarness(t)
	wantReason(t, h.run(open("code/p/a/README", "O_WRONLY|O_TRUNC", abs("code/p/a/README"))), "writes-tracked:code/p/a/README")
}

func TestRootKindDefaultIsHostRead(t *testing.T) {
	h := newHarness(t)
	h.cfg.HostRoots.Roots = append(h.cfg.HostRoots.Roots, h.cfg.HostRoots.Roots[0])
	h.cfg.HostRoots.Roots[len(h.cfg.HostRoots.Roots)-1].Path = "/odd"
	h.cfg.HostRoots.Roots[len(h.cfg.HostRoots.Roots)-1].Kind = "mystery"
	wantReason(t, h.run(openAbs("/odd/x")), "host-read:/odd/x")
}

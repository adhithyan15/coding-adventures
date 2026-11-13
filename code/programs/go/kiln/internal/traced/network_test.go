package traced

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// sockEv is a socket() call that returned the descriptor named path.
func sockEv(path, typ string) trace.Event {
	e := base("socket", trace.KindNetwork, "")
	e.Addr = &trace.Addr{Family: "AF_INET", Port: -1}
	e.Flags = "AF_INET, " + typ + "|SOCK_CLOEXEC, IPPROTO_IP"
	e.SockPath = path
	e.Result.Value = 3
	return e
}

// runNet runs events after a stream socket() named socket:[1].
func (h *harness) runNet(events ...trace.Event) *RawRecord {
	return h.run(append([]trace.Event{sockEv("socket:[1]", "SOCK_STREAM")}, events...)...)
}

func withPid(e trace.Event, pid int) trace.Event {
	e.Pid = pid
	return e
}

func TestNetworkPublicAddressWhateverItReturned(t *testing.T) {
	h := newHarness(t)
	// A non-blocking connect to a public address gets EINPROGRESS; judged by
	// address, not by result.
	e := fail(inet("connect", "93.184.216.34", 443), "EINPROGRESS")
	tr := step(e)
	tr.Procs[1].LastExec = "/usr/bin/curl"
	rec := h.runSteps(tr)
	eq(t, "reasons", rec.Reasons, "network:/usr/bin/curl")

	for name, ev := range map[string]trace.Event{
		"success":         inet("connect", "8.8.8.8", 443),
		"refused":         fail(inet("connect", "8.8.8.8", 443), "ECONNREFUSED"),
		"dns on loopback": inet("sendto", "127.0.0.1", 53),
		"dns v6":          withAddr(inet("sendmsg", "::1", 53), "AF_INET6"),
		"v6 public":       withAddr(inet("connect", "2001:db8::1", 80), "AF_INET6"),
		"garbage addr":    inet("connect", "not-an-ip", 80),
	} {
		rec := h.runNet(ev)
		if len(rec.Reasons) != 1 || rec.Reasons[0] != "network:unknown" {
			t.Errorf("%s: %q", name, rec.Reasons)
		}
	}
}

func withAddr(e trace.Event, family string) trace.Event {
	e.Addr = &trace.Addr{Family: family, Address: e.Addr.Address, Port: e.Addr.Port}
	return e
}

func TestNetworkProgramNameIsResolvedAndRedacted(t *testing.T) {
	h := newHarness(t)
	tr := step(inet("connect", "8.8.8.8", 80))
	tr.Procs[1].LastExec = "./tool"
	rec := h.runSteps(tr)
	eq(t, "reasons", rec.Reasons, "network:/w/repo/tool")
	tr.Procs[1].LastExec = "/home/runner/bin/fetch"
	rec = h.runSteps(tr)
	eq(t, "reasons", rec.Reasons, "network:$HOME/bin/fetch")
}

func TestNetworkLoopbackServerTheActionStarted(t *testing.T) {
	h := newHarness(t)
	rec := h.runNet(
		inet("bind", "127.0.0.1", 8080),
		inet("connect", "127.0.0.1", 8080),
		fail(inet("connect", "127.0.0.1", 8080), "EINPROGRESS"),
		inet("sendto", "127.0.0.1", 8080),
	)
	wantNoReasons(t, rec)
	// Wildcard bind serves loopback; port 0 stands for any port; v6 wildcard
	// serves the v4 loopback address too, but ::1 is not 127.0.0.1.
	rec = h.runNet(inet("bind", "0.0.0.0", 9000), inet("connect", "127.0.0.1", 9000))
	wantNoReasons(t, rec)
	// A bind to port 0 matches nothing: the port is not known (getsockname is
	// not parsed), so a client of that server reads as a daemon.
	rec = h.runNet(inet("bind", "127.0.0.1", 0), inet("connect", "127.0.0.1", 41234))
	eq(t, "reasons", rec.Reasons, "daemon:127.0.0.1:41234")
	rec = h.runNet(withAddr(inet("bind", "::", 9001), "AF_INET6"), inet("connect", "127.0.0.1", 9001))
	wantNoReasons(t, rec)
	rec = h.runNet(inet("bind", "127.0.0.1", 9002), withAddr(inet("connect", "::1", 9002), "AF_INET6"))
	eq(t, "reasons", rec.Reasons, "daemon:[::1]:9002")
	// Wildcards are family-scoped: 0.0.0.0 serves IPv4 only, :: serves both.
	rec = h.runNet(inet("bind", "0.0.0.0", 9004), withAddr(inet("connect", "::1", 9004), "AF_INET6"))
	eq(t, "reasons", rec.Reasons, "daemon:[::1]:9004")
	rec = h.runNet(withAddr(inet("bind", "::", 9005), "AF_INET6"), withAddr(inet("connect", "::1", 9005), "AF_INET6"))
	wantNoReasons(t, rec)
	rec = h.runNet(withAddr(inet("bind", "::1", 9006), "AF_INET6"), withAddr(inet("connect", "::", 9006), "AF_INET6"))
	wantNoReasons(t, rec)
	rec = h.runNet(inet("bind", "127.0.0.1", 9007), withAddr(inet("connect", "::", 9007), "AF_INET6"))
	eq(t, "reasons", rec.Reasons, "daemon:[::]:9007")
	// The socket type matters: a datagram socket is not answered by a stream server.
	dg := inet("connect", "127.0.0.1", 9008)
	dg.SockPath = "socket:[2]"
	rec = h.run(sockEv("socket:[1]", "SOCK_STREAM"), sockEv("socket:[2]", "SOCK_DGRAM"), inet("bind", "127.0.0.1", 9008), dg)
	eq(t, "reasons", rec.Reasons, "daemon:127.0.0.1:9008")
	// An unknown type (no socket() seen) matches nothing.
	unk := inet("connect", "127.0.0.1", 9009)
	unk.SockPath = "socket:[77]"
	rec = h.run(sockEv("socket:[1]", "SOCK_STREAM"), inet("bind", "127.0.0.1", 9009), unk)
	eq(t, "reasons", rec.Reasons, "daemon:127.0.0.1:9009")
	// A -y name that two socket() calls gave different types is unknown.
	rec = h.run(sockEv("socket:[1]", "SOCK_STREAM"), sockEv("socket:[1]", "SOCK_DGRAM"), inet("bind", "127.0.0.1", 9010), inet("connect", "127.0.0.1", 9010))
	eq(t, "reasons", rec.Reasons, "daemon:127.0.0.1:9010")
	// Connecting to the wildcard address means "this machine".
	rec = h.runNet(inet("bind", "127.0.0.1", 9003), inet("connect", "0.0.0.0", 9003))
	wantNoReasons(t, rec)
}

func TestNetworkExistingDaemon(t *testing.T) {
	h := newHarness(t)
	// Nothing here bound it: a daemon, success or in progress.
	eq(t, "reasons", h.runNet(inet("connect", "127.0.0.1", 5432)).Reasons, "daemon:127.0.0.1:5432")
	eq(t, "reasons", h.runNet(fail(inet("connect", "127.0.0.1", 5432), "EINPROGRESS")).Reasons, "daemon:127.0.0.1:5432")
	// A different port is not the one it bound; a failed bind binds nothing.
	eq(t, "reasons", h.runNet(inet("bind", "127.0.0.1", 1), inet("connect", "127.0.0.1", 2)).Reasons, "daemon:127.0.0.1:2")
	eq(t, "reasons", h.runNet(fail(inet("bind", "127.0.0.1", 7), "EADDRINUSE"), inet("connect", "127.0.0.1", 7)).Reasons, "daemon:127.0.0.1:7")
	// A bind that came AFTER the connect does not excuse it.
	eq(t, "reasons", h.runNet(inet("connect", "127.0.0.1", 7), inet("bind", "127.0.0.1", 7)).Reasons, "daemon:127.0.0.1:7")
}

func TestNetworkConnectionRefusedIsAllowed(t *testing.T) {
	h := newHarness(t)
	wantNoReasons(t, h.runNet(fail(inet("connect", "127.0.0.1", 5432), "ECONNREFUSED")))
	wantNoReasons(t, h.runNet(fail(unix("connect", "/run/docker.sock"), "ENOENT")))
	wantNoReasons(t, h.runNet(fail(unix("connect", "/run/docker.sock"), "ECONNREFUSED")))
	// Another failure is not an expected one.
	eq(t, "reasons", h.runNet(fail(inet("connect", "127.0.0.1", 5432), "EPERM")).Reasons, "daemon:127.0.0.1:5432")
}

func TestNetworkUnixSockets(t *testing.T) {
	h := newHarness(t)
	h.fs.file(abs("code/p/a/test.sock"))
	// A socket the action bound (inside the repository: created, a write).
	rec := h.runNet(unix("bind", "code/p/a/test.sock"), unix("connect", "code/p/a/test.sock"))
	wantNoReasons(t, rec)
	eq(t, "writes", rec.Writes, "code/p/a/test.sock")
	eq(t, "entries", rec.Entries, "pkg:code/p/a") // the absence the bind proved
	// A temp socket, relative to a different directory, still matches.
	e := unix("connect", "s.sock")
	e.Cwd = "/tmp/kiln-1"
	b := unix("bind", "/tmp/kiln-1/s.sock")
	wantNoReasons(t, h.runNet(b, e))
	// An existing socket someone else made: a daemon.
	eq(t, "reasons", h.runNet(unix("connect", "/var/run/docker.sock")).Reasons, "daemon:/var/run/docker.sock")
	wantReason(t, h.runNet(unix("sendto", "/home/runner/.s")), "daemon:$HOME/.s")
	// Inside the repository the socket file is observed, and a daemon too.
	rec = h.runNet(unix("connect", "code/p/b/live.sock"))
	eq(t, "reasons", rec.Reasons, "daemon:/w/repo/code/p/b/live.sock")
	eq(t, "entries", rec.Entries, "pkg:code/p/b")
	rec = h.runNet(fail(unix("connect", "code/p/b/live.sock"), "ENOENT"))
	wantNoReasons(t, rec)
	eq(t, "entries", rec.Entries, "pkg:code/p/b")
	// A failed bind is an observation: the path was there.
	rec = h.runNet(fail(unix("bind", "code/p/b/taken.sock"), "EADDRINUSE"))
	eq(t, "entries", rec.Entries, "pkg:code/p/b")
	eq(t, "writes", rec.Writes)
	// An unnamed socket has no peer.
	wantNoReasons(t, h.runNet(unix("sendto", ""), unix("bind", "")))
}

func TestNetworkAbstractSockets(t *testing.T) {
	h := newHarness(t)
	abstract := func(sys, name string) trace.Event {
		e := unix(sys, name)
		e.Addr.Abstract = true
		return e
	}
	wantNoReasons(t, h.runNet(abstract("bind", "mine"), abstract("connect", "mine")))
	eq(t, "reasons", h.runNet(abstract("connect", "dbus")).Reasons, "daemon:@dbus")
	wantNoReasons(t, h.runNet(fail(abstract("connect", "dbus"), "ECONNREFUSED")))
	// A failed abstract bind binds nothing.
	eq(t, "reasons", h.runNet(fail(abstract("bind", "x"), "EADDRINUSE"), abstract("connect", "x")).Reasons, "daemon:@x")
}

func TestNetworkOtherFamilies(t *testing.T) {
	h := newHarness(t)
	sock := func(family string) trace.Event {
		e := base("socket", trace.KindNetwork, "")
		e.Addr = &trace.Addr{Family: family, Port: -1}
		return e
	}
	for _, f := range []string{"AF_NETLINK", "AF_PACKET", "AF_VSOCK"} {
		eq(t, f, h.runNet(sock(f)).Reasons, "network:unknown")
	}
	for _, f := range []string{"AF_UNIX", "AF_INET", "AF_INET6"} {
		wantNoReasons(t, h.runNet(sock(f)))
	}
	// A connect to an address in another family is network use too.
	e := unix("connect", "")
	e.Addr = &trace.Addr{Family: "AF_NETLINK", Port: -1}
	eq(t, "reasons", h.runNet(e).Reasons, "network:unknown")
	// listen, and sends on a connected socket, carry no address.
	l := base("listen", trace.KindNetwork, "")
	s := base("sendto", trace.KindNetwork, "")
	wantNoReasons(t, h.runNet(l, s))
}

func TestNetworkProgramWithoutExec(t *testing.T) {
	h := newHarness(t)
	tr := step(withPid(inet("connect", "8.8.8.8", 80), 5))
	rec := h.runSteps(tr)
	eq(t, "reasons", rec.Reasons, "network:unknown")
}

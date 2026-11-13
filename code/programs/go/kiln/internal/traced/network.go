package traced

import (
	"net/netip"
	"path"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// Network (KILN04 section 5.4).
//
// Network use is judged by ADDRESS, never by return value: curl, Node, Go and
// asyncio all get EINPROGRESS from a non-blocking connect, and an answer that
// depended on whether the connection worked would depend on the weather.
//
//	connect/sendto/sendmsg/sendmmsg to
//	  a non-loopback address, or port 53 anywhere      -> network:<program>
//	  loopback or Unix the action itself bound         -> allowed (a test server)
//	  loopback or Unix nobody here bound               -> daemon:<address>
//	  ... that failed with ECONNREFUSED or ENOENT      -> allowed (host state
//	                                                      the image fixes)
//	any socket outside AF_UNIX, AF_INET, AF_INET6      -> network:<program>
//	abstract Unix socket the action did not bind       -> daemon:<address>
//
// "Bound" means a successful bind() earlier in the trace, in any step of the
// action, on a socket of the SAME TYPE (a stream server does not answer a
// datagram, and the socket()'s type is read from its -y name).  A bind to port
// 0 asked the kernel for any free port, and strace's bind line does not say
// which: it matches nothing (learning the port from getsockname is future
// work), so such a server's clients read as daemons and the record is refused.
// A wildcard bind is family-scoped: 0.0.0.0 serves IPv4 only, :: serves both.

type boundAddr struct {
	ip   netip.Addr
	port int
	typ  string
}

// sockType returns the socket type (SOCK_STREAM, ...) of the socket the event
// used, or "" when it is not known.
func (c *classifier) sockType(ev *trace.Event) string {
	t := c.sockTypes[ev.SockPath]
	if t == "?" {
		return ""
	}
	return t
}

// noteSocket reads the type from a socket() call: "AF_INET, SOCK_STREAM|SOCK_CLOEXEC, 0".
// Two sockets that share a -y name with different types make it unknown.
func (c *classifier) noteSocket(ev *trace.Event) {
	if ev.SockPath == "" || !ev.Result.OK {
		return
	}
	parts := strings.Split(ev.Flags, ", ")
	if len(parts) < 2 {
		return
	}
	typ, _, _ := strings.Cut(parts[1], "|")
	if old, ok := c.sockTypes[ev.SockPath]; ok && old != typ {
		typ = "?"
	}
	c.sockTypes[ev.SockPath] = typ
}

// network handles one network event.
func (c *classifier) network(t *trace.Trace, ev *trace.Event) {
	a := ev.Addr
	if a == nil {
		if ev.Syscall == "bind" {
			c.incomplete = true // W4: a bind with no address may have made a socket file
		}
		return // a connected socket sending, or listen: the connect was judged
	}
	switch ev.Syscall {
	case "socket":
		c.noteSocket(ev)
		if !allowedFamily(a.Family) {
			c.networkUse(t, ev)
		}
		return
	case "bind":
		c.bind(ev)
		return
	case "listen":
		return
	}
	// connect, sendto, sendmsg, sendmmsg
	switch a.Family {
	case "AF_INET", "AF_INET6":
		c.inetPeer(t, ev)
	case "AF_UNIX":
		c.unixPeer(ev)
	default:
		c.networkUse(t, ev)
	}
}

func allowedFamily(f string) bool {
	return f == "AF_UNIX" || f == "AF_INET" || f == "AF_INET6"
}

// networkUse reports network use by the process that made the call.
func (c *classifier) networkUse(t *trace.Trace, ev *trace.Event) {
	prog := "unknown"
	if p := t.Procs[ev.Pid]; p != nil && p.LastExec != "" {
		prog = p.LastExec
		if !strings.HasPrefix(prog, "/") {
			prog = path.Clean(join(ev.Cwd, "", prog))
		}
	}
	c.reason("network:" + c.redact(prog))
}

// localAddr is loopback, or the wildcard address, which a connect treats as
// "this machine".
func localAddr(ip netip.Addr) bool { return ip.IsLoopback() || ip.IsUnspecified() }

func (c *classifier) bound(ip netip.Addr, port int, typ string) bool {
	if typ == "" {
		return false
	}
	for _, b := range c.boundInet {
		if b.port == port && b.typ == typ && addrServes(b.ip, ip) {
			return true
		}
	}
	return false
}

// addrServes says whether a socket bound to b answers a connect to ip.
//
//	bound ::         serves every address of both families (dual stack)
//	bound 0.0.0.0    serves IPv4 addresses only
//	connect to ::    means this machine over IPv6: a bound loopback or wildcard IPv6 socket
//	connect to 0.0.0.0   the same over IPv4
//	otherwise        the same address (a v4-mapped v6 address is its v4 address)
func addrServes(b, ip netip.Addr) bool {
	b, ip = b.Unmap(), ip.Unmap()
	if b.IsUnspecified() {
		return b.Is6() || ip.Is4()
	}
	if ip.IsUnspecified() {
		return b.Is4() == ip.Is4() && b.IsLoopback()
	}
	return b == ip
}

// expectedFailure is the failure of a connect or send to a local peer that
// observes only host state the image fixes.
func expectedFailure(r trace.Result) bool {
	return !r.OK && (r.Errno == "ECONNREFUSED" || r.Errno == "ENOENT")
}

func (c *classifier) inetPeer(t *trace.Trace, ev *trace.Event) {
	a := ev.Addr
	ip, err := netip.ParseAddr(a.Address)
	if err != nil || a.Port == 53 || !localAddr(ip) {
		c.networkUse(t, ev)
		return
	}
	if expectedFailure(ev.Result) || c.bound(ip, a.Port, c.sockType(ev)) {
		return
	}
	// A loopback peer nobody here bound lives in the step's own network
	// namespace (KILN04 4.7), so it cannot be a host daemon: only the reason.
	c.reason("daemon:" + sanitize(addrText(ip, a.Port)))
}

func addrText(ip netip.Addr, port int) string {
	return netip.AddrPortFrom(ip, uint16(port)).String()
}

// bind records a successful bind.  An AF_UNIX pathname bind also CREATES the
// socket file, so it is the action's own path and a write.
func (c *classifier) bind(ev *trace.Event) {
	a := ev.Addr
	switch a.Family {
	case "AF_INET", "AF_INET6":
		if !ev.Result.OK {
			return
		}
		if ip, err := netip.ParseAddr(a.Address); err == nil && a.Port != 0 {
			c.boundInet = append(c.boundInet, boundAddr{ip, a.Port, c.sockType(ev)})
			c.binds[c.sockType(ev)+"/"+addrText(ip, a.Port)] = true
		}
	case "AF_UNIX":
		if a.Abstract {
			if ev.Result.OK {
				c.boundUnix["@"+a.Address] = c.sockType(ev)
			}
			return
		}
		if a.Address == "" {
			return
		}
		c.writing(func() { c.bindPath(ev, a.Address) })
	}
}

// bindPath places the socket file of an AF_UNIX pathname bind.
func (c *classifier) bindPath(ev *trace.Event, addr string) {
	tg, ok := c.resolve(join(ev.Cwd, "", addr), false)
	if !ok {
		c.incomplete = true // W4: a bind that could not be placed
		return
	}
	if tg.virtual {
		c.incomplete = true // W5
		return
	}
	if !ev.Result.OK {
		c.observeResult(tg, ev.Result)
		return
	}
	c.writePath(tg.abs)
	c.markCreated(tg.abs, true) // bind never replaces a name: like O_EXCL
	c.boundUnix[tg.abs] = c.sockType(ev)
}

// unixPeer handles a connect or send to an AF_UNIX address.
func (c *classifier) unixPeer(ev *trace.Event) {
	a := ev.Addr
	if a.Abstract {
		if expectedFailure(ev.Result) || c.unixBound("@"+a.Address, c.sockType(ev)) {
			return
		}
		// The abstract namespace is per network namespace too: only the reason.
		c.reason("daemon:" + sanitize("@"+a.Address))
		return
	}
	if a.Address == "" {
		return // an unnamed socket has no peer to be a daemon
	}
	// The peer's lookups are the write-set's business too (L5): "the action
	// bound this address itself" is believed only while the bound name is
	// stable, so an action that swaps the name for a link to a host daemon's
	// socket between bind and connect (and back) is judged by the same
	// stability rule as a write, and is incomplete.
	var tg target
	var ok bool
	c.writing(func() { tg, ok = c.resolve(join(ev.Cwd, "", a.Address), true) })
	if !ok {
		return
	}
	// Inside the repository the socket file is a path like any other, and its
	// absence or presence is an observation.  Outside it, a failed connect
	// observes only host state the image fixes, and is allowed as it is.
	if _, in := c.rel(tg.abs); in {
		c.observeResult(tg, ev.Result)
	}
	if expectedFailure(ev.Result) || c.unixBound(tg.abs, c.sockType(ev)) {
		return
	}
	c.reason("daemon:" + c.redact(tg.abs))
	// L2 (M4): a connect that SUCCEEDED to a pathname socket outside the
	// repository and outside the fresh HOME and TMPDIR reached a host
	// process, which may write the repository for the action.  Peers inside
	// the repository or the scratch directories are another action's (the
	// cross-action rule M13 handles those).
	if _, in := c.rel(tg.abs); !in && ev.Result.OK && !c.underTmp(tg.abs) && !c.isTmpRoot(tg.abs) {
		c.incomplete = true
	}
}

// unixBound says the action bound this Unix address on a socket of this type.
func (c *classifier) unixBound(addr, typ string) bool {
	t, ok := c.boundUnix[addr]
	return ok && typ != "" && t == typ
}

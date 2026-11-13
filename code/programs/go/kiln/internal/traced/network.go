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
// action.  A bind to port 0 asked the kernel for any free port, and strace's
// bind line does not say which, so it stands for every port of that address.

type boundAddr struct {
	ip   netip.Addr
	port int
}

// network handles one network event.
func (c *classifier) network(t *trace.Trace, ev *trace.Event) {
	a := ev.Addr
	if a == nil {
		return // a connected socket sending, or listen: the connect was judged
	}
	switch ev.Syscall {
	case "socket":
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

func (c *classifier) bound(ip netip.Addr, port int) bool {
	for _, b := range c.boundInet {
		if b.port != port && b.port != 0 {
			continue
		}
		if b.ip.Unmap() == ip.Unmap() || b.ip.IsUnspecified() || ip.IsUnspecified() {
			return true
		}
	}
	return false
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
	if expectedFailure(ev.Result) || c.bound(ip, a.Port) {
		return
	}
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
		if ip, err := netip.ParseAddr(a.Address); err == nil {
			c.boundInet = append(c.boundInet, boundAddr{ip, a.Port})
		}
	case "AF_UNIX":
		if a.Abstract {
			if ev.Result.OK {
				c.boundUnix["@"+a.Address] = true
			}
			return
		}
		if a.Address == "" {
			return
		}
		tg, ok := c.resolve(join(ev.Cwd, "", a.Address), false)
		if !ok {
			return
		}
		if !ev.Result.OK {
			c.observeResult(tg.abs, ev.Result)
			return
		}
		c.writePath(tg.abs)
		c.markCreated(tg.abs)
		c.boundUnix[tg.abs] = true
	}
}

// unixPeer handles a connect or send to an AF_UNIX address.
func (c *classifier) unixPeer(ev *trace.Event) {
	a := ev.Addr
	if a.Abstract {
		if expectedFailure(ev.Result) || c.boundUnix["@"+a.Address] {
			return
		}
		c.reason("daemon:" + sanitize("@"+a.Address))
		return
	}
	if a.Address == "" {
		return // an unnamed socket has no peer to be a daemon
	}
	tg, ok := c.resolve(join(ev.Cwd, "", a.Address), true)
	if !ok {
		return
	}
	// Inside the repository the socket file is a path like any other, and its
	// absence or presence is an observation.  Outside it, a failed connect
	// observes only host state the image fixes, and is allowed as it is.
	if _, in := c.rel(tg.abs); in {
		c.observeResult(tg.abs, ev.Result)
	}
	if expectedFailure(ev.Result) || c.boundUnix[tg.abs] {
		return
	}
	c.reason("daemon:" + c.redact(tg.abs))
}

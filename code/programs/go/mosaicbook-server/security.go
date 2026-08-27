// security.go — Origin/Host validation guarding this server's HTTP surface
//
// mosaicbook-server binds to localhost only, but binding to localhost does
// not, by itself, stop anything already running on this machine from
// reaching it: a page open in the developer's own browser (from any
// website that can guess or scan the port) can issue a same-machine
// cross-origin request, and DNS rebinding lets an attacker-controlled
// domain resolve to 127.0.0.1 after the browser's same-origin checks have
// already passed for that domain. Either way, the Host header the server
// actually receives still names the untrusted domain, not "localhost" —
// checking Host (not just the bind address) is what closes this. See #13178,
// filed during #12027's drop-panel review: GET /api/degradations spawns a
// real mosaic-compile subprocess per request, turning a drive-by
// cross-origin GET into repeated build invocations rather than just static
// bytes.
//
// requireLocalOrigin wraps the whole mux once, in main.go, rather than
// individual routes — a route added later is protected without anyone
// remembering to wrap it, and every existing handler-level test (which
// calls srv.mux.ServeHTTP directly, bypassing this wrapper entirely) is
// unaffected, since the guard sits outside the mux rather than baked into
// it.

package main

import (
	"net"
	"net/http"
	"net/url"
)

// isLocalHost reports whether hostPort — a Host header value or an Origin
// URL's Host component, either of which may or may not carry a :port
// suffix — names this machine's loopback interface.
func isLocalHost(hostPort string) bool {
	hostname := hostPort
	if h, _, err := net.SplitHostPort(hostPort); err == nil {
		hostname = h
	}
	switch hostname {
	case "localhost", "127.0.0.1", "::1":
		return true
	}
	return false
}

// requireLocalOrigin rejects any request whose Host header doesn't name
// this machine's loopback interface. Two further, independent signals are
// checked to catch a same-machine page reaching this server directly rather
// than being it:
//
//   - When an Origin header is present (as it always is for fetch()/XHR/
//     img/script), it must also name loopback.
//   - When the Sec-Fetch-Site header is present (sent by every modern
//     browser, for every request type — critically, including navigations,
//     which is where this matters most: GET /preview/... is designed to be
//     loaded via <iframe src=...>, and a cross-origin *navigation* carries
//     no Origin header at all per the Fetch spec, so the Origin check alone
//     is silently skipped for exactly the request shape this tool's own
//     iframe embedding uses). Sec-Fetch-Site's value is "cross-site" for a
//     request whose initiating page is on a different site, regardless of
//     request type — it is what actually catches a hostile page's
//     <iframe src="http://localhost:PORT/preview/...">.
//
// A request with neither header present — a plain curl, or a same-origin
// load in a browser old enough to lack Fetch Metadata support — is allowed
// through on the Host check alone, same as before either signal existed.
func requireLocalOrigin(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !isLocalHost(r.Host) {
			http.Error(w, "forbidden: this server only accepts requests addressed to localhost", http.StatusForbidden)
			return
		}
		if site := r.Header.Get("Sec-Fetch-Site"); site == "cross-site" {
			http.Error(w, "forbidden: cross-site request rejected", http.StatusForbidden)
			return
		}
		if origin := r.Header.Get("Origin"); origin != "" {
			u, err := url.Parse(origin)
			if err != nil || !isLocalHost(u.Host) {
				http.Error(w, "forbidden: cross-origin request rejected", http.StatusForbidden)
				return
			}
		}
		next.ServeHTTP(w, r)
	})
}

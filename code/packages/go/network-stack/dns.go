package networkstack

// DNS — Domain Name System
//
// DNS is the Internet's phone book. When you type "google.com", your computer
// doesn't know how to reach it — it needs an IP address like 142.250.80.46.
// DNS translates human-readable hostnames into IP addresses.
//
// Our implementation uses a simple static lookup table. Real DNS involves
// UDP queries to recursive resolvers, caching with TTLs, and complex record
// types (A, AAAA, CNAME, MX).
//
// The default entry maps "localhost" to 127.0.0.1 (0x7F000001), which is
// the loopback address — it always refers to the local machine.

// DNSResolver provides hostname-to-IP resolution using static mappings.
type DNSResolver struct {
	static map[string]uint32
}

// NewDNSResolver creates a resolver pre-populated with localhost -> 127.0.0.1.
func NewDNSResolver() *DNSResolver {
	return &DNSResolver{
		static: map[string]uint32{
			"localhost": 0x7F000001,
		},
	}
}

// AddStatic registers a hostname-to-IP mapping (like /etc/hosts).
func (r *DNSResolver) AddStatic(hostname string, ip uint32) {
	r.static[hostname] = ip
}

// Resolve looks up a hostname. Returns the IP and whether it was found.
func (r *DNSResolver) Resolve(hostname string) (uint32, bool) {
	ip, ok := r.static[hostname]
	return ip, ok
}

// Entries returns a copy of all static DNS entries.
func (r *DNSResolver) Entries() map[string]uint32 {
	result := make(map[string]uint32, len(r.static))
	for k, v := range r.static {
		result[k] = v
	}
	return result
}

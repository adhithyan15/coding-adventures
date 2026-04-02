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
	result, _ := StartNew[*DNSResolver]("network-stack.NewDNSResolver", nil,
		func(op *Operation[*DNSResolver], rf *ResultFactory[*DNSResolver]) *OperationResult[*DNSResolver] {
			return rf.Generate(true, false, &DNSResolver{
				static: map[string]uint32{
					"localhost": 0x7F000001,
				},
			})
		}).GetResult()
	return result
}

// AddStatic registers a hostname-to-IP mapping (like /etc/hosts).
func (r *DNSResolver) AddStatic(hostname string, ip uint32) {
	_, _ = StartNew[struct{}]("network-stack.AddStatic", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("hostname", hostname)
			r.static[hostname] = ip
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Resolve looks up a hostname. Returns the IP and whether it was found.
func (r *DNSResolver) Resolve(hostname string) (uint32, bool) {
	var found bool
	ip, _ := StartNew[uint32]("network-stack.Resolve", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("hostname", hostname)
			v, ok := r.static[hostname]
			found = ok
			return rf.Generate(true, false, v)
		}).GetResult()
	return ip, found
}

// Entries returns a copy of all static DNS entries.
func (r *DNSResolver) Entries() map[string]uint32 {
	result, _ := StartNew[map[string]uint32]("network-stack.Entries", nil,
		func(op *Operation[map[string]uint32], rf *ResultFactory[map[string]uint32]) *OperationResult[map[string]uint32] {
			copy := make(map[string]uint32, len(r.static))
			for k, v := range r.static {
				copy[k] = v
			}
			return rf.Generate(true, false, copy)
		}).GetResult()
	return result
}

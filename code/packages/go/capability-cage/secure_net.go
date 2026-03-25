// Secure network wrappers.
//
// These functions are drop-in replacements for net.Dial, net.Listen, and
// net.LookupHost, with a capability check injected before delegation.
//
// Target formats by action:
//   - connect: "host:port" (e.g., "api.example.com:443")
//   - listen:  "host:port" or ":port" for all interfaces (e.g., ":8080")
//   - dns:     hostname (e.g., "api.example.com")
//
// Use "*" as the target to permit connections to any address.
package capabilitycage

import "net"

// Connect checks net:connect:{address} against m, then dials the address.
//
// network is the network type: "tcp", "tcp4", "tcp6", "udp", etc.
// address is "host:port" (e.g., "api.example.com:443").
//
// Returns CapabilityViolationError if the manifest does not declare
// net:connect for the given address.
func Connect(m *Manifest, network, address string) (net.Conn, error) {
	if err := m.Check(CategoryNet, ActionConnect, address); err != nil {
		return nil, err
	}
	return defaultBackend.Dial(network, address)
}

// Listen checks net:listen:{address} against m, then binds and listens.
//
// network is the network type: "tcp", "tcp4", "tcp6", "unix", etc.
// address is "host:port" or ":port" (e.g., ":8080" for all interfaces).
//
// Returns CapabilityViolationError if the manifest does not declare
// net:listen for the given address.
func Listen(m *Manifest, network, address string) (net.Listener, error) {
	if err := m.Check(CategoryNet, ActionListen, address); err != nil {
		return nil, err
	}
	return defaultBackend.Listen(network, address)
}

// DNSLookup checks net:dns:{host} against m, then resolves the hostname.
//
// Returns a slice of IP address strings for the given host.
// Returns CapabilityViolationError if the manifest does not declare
// net:dns for the given host.
func DNSLookup(m *Manifest, host string) ([]string, error) {
	if err := m.Check(CategoryNet, ActionDNS, host); err != nil {
		return nil, err
	}
	return defaultBackend.LookupHost(host)
}

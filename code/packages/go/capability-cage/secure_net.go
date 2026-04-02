// Secure network wrappers.
package capabilitycage

import "net"

// Connect checks net:connect:{address} against m, then dials the address.
func Connect(m *Manifest, network, address string) (net.Conn, error) {
	return StartNew[net.Conn]("capability-cage.Connect", nil,
		func(op *Operation[net.Conn], rf *ResultFactory[net.Conn]) *OperationResult[net.Conn] {
			op.AddProperty("network", network)
			op.AddProperty("address", address)
			if err := m.Check(CategoryNet, ActionConnect, address); err != nil {
				return rf.Fail(nil, err)
			}
			conn, err := defaultBackend.Dial(network, address)
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, conn)
		}).GetResult()
}

// Listen checks net:listen:{address} against m, then binds and listens.
func Listen(m *Manifest, network, address string) (net.Listener, error) {
	return StartNew[net.Listener]("capability-cage.Listen", nil,
		func(op *Operation[net.Listener], rf *ResultFactory[net.Listener]) *OperationResult[net.Listener] {
			op.AddProperty("network", network)
			op.AddProperty("address", address)
			if err := m.Check(CategoryNet, ActionListen, address); err != nil {
				return rf.Fail(nil, err)
			}
			ln, err := defaultBackend.Listen(network, address)
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, ln)
		}).GetResult()
}

// DNSLookup checks net:dns:{host} against m, then resolves the hostname.
func DNSLookup(m *Manifest, host string) ([]string, error) {
	return StartNew[[]string]("capability-cage.DNSLookup", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			op.AddProperty("host", host)
			if err := m.Check(CategoryNet, ActionDNS, host); err != nil {
				return rf.Fail(nil, err)
			}
			addrs, err := defaultBackend.LookupHost(host)
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, addrs)
		}).GetResult()
}

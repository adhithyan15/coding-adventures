package main

import "testing"

// The server listens on loopback only: another machine on the network could
// otherwise reach it by sending "Host: localhost", which the Host check
// accepts.
func TestListenAddressIsLoopbackOnly(t *testing.T) {
	if got := listenAddress(7331); got != "127.0.0.1:7331" {
		t.Fatalf("listenAddress(7331) = %q, want 127.0.0.1:7331", got)
	}
}

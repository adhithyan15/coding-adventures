package networkstack

import "testing"

func TestDNSLocalhostDefault(t *testing.T) {
	r := NewDNSResolver()
	ip, ok := r.Resolve("localhost")
	if !ok {
		t.Fatal("localhost should be resolved")
	}
	if ip != 0x7F000001 {
		t.Errorf("expected 0x7F000001, got 0x%08x", ip)
	}
}

func TestDNSAddAndResolve(t *testing.T) {
	r := NewDNSResolver()
	r.AddStatic("myserver.local", 0x0A000001)
	ip, ok := r.Resolve("myserver.local")
	if !ok || ip != 0x0A000001 {
		t.Errorf("resolution failed")
	}
}

func TestDNSUnknownReturnsNotFound(t *testing.T) {
	r := NewDNSResolver()
	_, ok := r.Resolve("unknown.host")
	if ok {
		t.Error("expected not found")
	}
}

func TestDNSOverwrite(t *testing.T) {
	r := NewDNSResolver()
	r.AddStatic("host", 0x01)
	r.AddStatic("host", 0x02)
	ip, _ := r.Resolve("host")
	if ip != 0x02 {
		t.Errorf("expected overwritten value")
	}
}

func TestDNSMultipleEntries(t *testing.T) {
	r := NewDNSResolver()
	r.AddStatic("a", 0x01)
	r.AddStatic("b", 0x02)
	ip1, _ := r.Resolve("a")
	ip2, _ := r.Resolve("b")
	if ip1 != 0x01 || ip2 != 0x02 {
		t.Errorf("entries should be independent")
	}
}

func TestDNSEntriesCopy(t *testing.T) {
	r := NewDNSResolver()
	entries := r.Entries()
	entries["hacked"] = 0xDEAD
	_, ok := r.Resolve("hacked")
	if ok {
		t.Error("modifying copy should not affect resolver")
	}
}

func TestDNSCaseSensitive(t *testing.T) {
	r := NewDNSResolver()
	r.AddStatic("MyServer", 0x01)
	_, ok := r.Resolve("myserver")
	if ok {
		t.Error("lookups should be case-sensitive")
	}
}

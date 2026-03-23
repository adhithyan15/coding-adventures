"""Tests for the DNS resolver."""

from network_stack.dns import DNSResolver


class TestDNSResolver:
    """Tests for static DNS resolution."""

    def test_localhost_default(self) -> None:
        """The resolver should have localhost -> 127.0.0.1 by default."""
        resolver = DNSResolver()
        ip = resolver.resolve("localhost")
        assert ip == 0x7F000001  # 127.0.0.1

    def test_add_static_and_resolve(self) -> None:
        """add_static should register a hostname for resolution."""
        resolver = DNSResolver()
        resolver.add_static("myserver.local", 0x0A000001)
        assert resolver.resolve("myserver.local") == 0x0A000001

    def test_resolve_unknown_returns_none(self) -> None:
        """Resolving an unknown hostname should return None."""
        resolver = DNSResolver()
        assert resolver.resolve("unknown.host") is None

    def test_overwrite_entry(self) -> None:
        """Adding the same hostname again should overwrite the old IP."""
        resolver = DNSResolver()
        resolver.add_static("host.local", 0x01)
        resolver.add_static("host.local", 0x02)
        assert resolver.resolve("host.local") == 0x02

    def test_multiple_entries(self) -> None:
        """Multiple hostnames should be tracked independently."""
        resolver = DNSResolver()
        resolver.add_static("a.local", 0x01)
        resolver.add_static("b.local", 0x02)
        assert resolver.resolve("a.local") == 0x01
        assert resolver.resolve("b.local") == 0x02

    def test_entries_returns_copy(self) -> None:
        """entries() should return a copy of the static table."""
        resolver = DNSResolver()
        entries = resolver.entries()
        assert "localhost" in entries
        # Modifying the copy should not affect the resolver
        entries["hacked"] = 0xDEAD
        assert resolver.resolve("hacked") is None

    def test_case_sensitive(self) -> None:
        """DNS lookups should be case-sensitive (simplified behavior)."""
        resolver = DNSResolver()
        resolver.add_static("MyServer", 0x01)
        assert resolver.resolve("MyServer") == 0x01
        assert resolver.resolve("myserver") is None

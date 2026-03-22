"""
DNS — Domain Name System
=========================

DNS is the Internet's phone book. When you type "google.com" in your browser,
your computer doesn't know how to reach "google.com" — it needs an IP address
like 142.250.80.46. DNS translates human-readable hostnames into IP addresses.

How DNS Works (Simplified)
--------------------------

1. You type "example.com" in your browser.
2. Your OS checks its **local DNS cache** — has it resolved this name recently?
3. If not cached, it sends a DNS query to a **recursive resolver** (usually
   your ISP's DNS server, or 8.8.8.8 for Google's public DNS).
4. The resolver queries the DNS hierarchy:
   - Root servers ("who handles .com?")
   - TLD servers ("who handles example.com?")
   - Authoritative servers ("what's the IP for example.com?")
5. The answer comes back: "example.com = 93.184.216.34"
6. Your OS caches the result and connects to that IP.

Our Implementation
------------------

We implement a simplified DNS resolver with a static lookup table. Real DNS
involves UDP packets, caching with TTLs, recursive queries, and complex
record types (A, AAAA, CNAME, MX, etc.). Our version captures the essential
concept: hostname -> IP address mapping.

The default entry maps "localhost" to 127.0.0.1 (0x7F000001), which is the
**loopback address** — it always refers to the local machine.
"""

from __future__ import annotations


class DNSResolver:
    """
    A simple DNS resolver using static hostname-to-IP mappings.

    In a real OS, DNS resolution involves sending UDP packets to a DNS
    server (typically on port 53) and parsing the response. Our simulation
    uses a dictionary for clarity.

    The resolver comes pre-loaded with "localhost" -> 127.0.0.1, which is
    the standard loopback mapping present on every operating system.

    Example
    -------
    >>> resolver = DNSResolver()
    >>> hex(resolver.resolve("localhost"))
    '0x7f000001'
    >>> resolver.add_static("myserver.local", 0x0A000001)
    >>> hex(resolver.resolve("myserver.local"))
    '0xa000001'
    >>> resolver.resolve("unknown.host") is None
    True
    """

    def __init__(self) -> None:
        # Pre-populate with localhost — the universal loopback address.
        # 0x7F000001 = 127.0.0.1 in network byte order.
        self._static: dict[str, int] = {"localhost": 0x7F000001}

    def add_static(self, hostname: str, ip: int) -> None:
        """
        Add a static DNS entry (like /etc/hosts on Unix).

        Parameters
        ----------
        hostname : str
            The hostname to register (e.g., "myserver.local").
        ip : int
            The IP address as a 32-bit integer.
        """
        self._static[hostname] = ip

    def resolve(self, hostname: str) -> int | None:
        """
        Resolve a hostname to an IP address.

        Looks up the hostname in the static table. Returns the IP as a
        32-bit integer, or None if the hostname is not found.

        In a real implementation, a cache miss would trigger a DNS query
        over the network (UDP port 53). Our simulation only uses static
        entries.
        """
        return self._static.get(hostname)

    def entries(self) -> dict[str, int]:
        """Return a copy of all static DNS entries."""
        return dict(self._static)

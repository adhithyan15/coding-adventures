"""
Network Stack — A Complete Layered Networking Implementation
============================================================

This package implements a full TCP/IP networking stack, from raw Ethernet frames
all the way up to HTTP requests. It is a teaching implementation: every layer is
transparent, every byte is visible, and every protocol transition is explicit.

The Layered Model
-----------------

Networking is organized in layers, each solving one problem:

    Layer 7: Application (HTTP, DNS)    — "What are we saying?"
    Layer 4: Transport (TCP, UDP)       — "How do we ensure delivery?"
    Layer 3: Network (IP)               — "How do we route across networks?"
    Layer 2: Data Link (Ethernet)       — "How do we talk to the next hop?"
    Layer 1: Physical (NetworkWire)     — "How do we transmit bits?"

Modules
-------

- ``ethernet``     — EthernetFrame, ARPTable (Layer 2)
- ``ipv4``         — IPv4Header, RoutingTable, IPLayer (Layer 3)
- ``tcp``          — TCPHeader, TCPConnection, TCPState (Layer 4)
- ``udp``          — UDPHeader, UDPSocket (Layer 4)
- ``socket_api``   — Socket, SocketManager (BSD socket interface)
- ``dns``          — DNSResolver (hostname to IP)
- ``http``         — HTTPRequest, HTTPResponse, HTTPClient (Layer 7)
- ``network_wire`` — NetworkWire (simulated physical medium)
"""

from network_stack.dns import DNSResolver
from network_stack.ethernet import ARPTable, EthernetFrame, ETHERTYPE_ARP, ETHERTYPE_IPV4
from network_stack.http import HTTPClient, HTTPRequest, HTTPResponse
from network_stack.ipv4 import IPLayer, IPv4Header, RoutingTable
from network_stack.network_wire import NetworkWire
from network_stack.socket_api import Socket, SocketManager, SocketType
from network_stack.tcp import (
    TCP_ACK,
    TCP_FIN,
    TCP_PSH,
    TCP_RST,
    TCP_SYN,
    TCPConnection,
    TCPHeader,
    TCPState,
)
from network_stack.udp import UDPHeader, UDPSocket

__all__ = [
    "EthernetFrame",
    "ARPTable",
    "ETHERTYPE_IPV4",
    "ETHERTYPE_ARP",
    "IPv4Header",
    "RoutingTable",
    "IPLayer",
    "TCPState",
    "TCPHeader",
    "TCPConnection",
    "TCP_FIN",
    "TCP_SYN",
    "TCP_RST",
    "TCP_PSH",
    "TCP_ACK",
    "UDPHeader",
    "UDPSocket",
    "Socket",
    "SocketManager",
    "SocketType",
    "DNSResolver",
    "HTTPRequest",
    "HTTPResponse",
    "HTTPClient",
    "NetworkWire",
]

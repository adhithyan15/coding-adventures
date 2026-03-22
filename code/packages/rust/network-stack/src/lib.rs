// ============================================================================
// Network Stack — Full Layered Networking
// ============================================================================
//
// This crate implements a complete network stack, from raw Ethernet frames
// (Layer 2) through HTTP (Layer 7). Each layer solves one problem and hides
// it from the layers above — the HTTP layer does not need to know whether
// bytes travel over WiFi, Ethernet, or carrier pigeon.
//
// The layers, bottom to top:
//
//   Layer 2: Ethernet  — local delivery using MAC addresses
//   Layer 3: IPv4      — routing across networks using IP addresses
//   Layer 4: TCP       — reliable, ordered byte streams
//   Layer 4: UDP       — fast, connectionless datagrams
//   Socket API         — Berkeley sockets interface for applications
//   Layer 7: DNS       — hostname-to-IP resolution
//   Layer 7: HTTP      — web request/response protocol
//   NetworkWire        — simulated physical medium (Ethernet cable)
//
// ============================================================================

pub mod ethernet;
pub mod ipv4;
pub mod tcp;
pub mod udp;
pub mod socket_api;
pub mod dns;
pub mod http;
pub mod network_wire;

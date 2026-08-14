# ssdp-protocol

`ssdp-protocol` owns strict, bounded framing for UPnP SSDP `M-SEARCH`
requests and unicast search responses. It validates the request target,
response status, required headers, cache lifetime, USN/UDN shape, optional
boot/configuration identifiers, duplicate headers, and body absence.

It performs no socket I/O, HTTP description fetch, event subscription, or
UPnP control action.

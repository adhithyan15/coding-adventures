# upnp-av-protocol

Strict fixed UPnP AV codec for one MediaRenderer instance. It parses a bounded
device description, requires AVTransport:1 and RenderingControl:1, emits only
the fixed state and media actions represented by `Action`, and validates the
matching SOAP response before returning typed playback, volume, mute, or track
metadata.

It does not expose arbitrary SOAP, ContentDirectory browse, connection
management, event subscriptions, queue mutation, URI transfer, seek, next or
previous, vendor extensions, or transport ownership.

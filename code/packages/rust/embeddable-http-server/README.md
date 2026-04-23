# embeddable-http-server

Embeddable HTTP/1 server primitive built on `tcp-runtime`.

This crate owns HTTP/1 request framing and response serialization while leaving
application work behind a handler callback. The eventual language bridges can
wrap this crate and map requests into Rack-style or WSGI-style application
objects without having to reimplement the socket runtime.

"""
coding_adventures.conduit — Flask-like Python web framework backed by Rust.

This package exposes the Conduit web framework: a Sinatra/Flask-style DSL
where routes are registered with decorators and response helpers (``ctx.json``,
``ctx.html``, ``ctx.halt``, ``ctx.redirect``) drive the response. The HTTP
engine — TCP, routing, HTTP/1 framing — lives in the Rust ``web-core`` crate.

Quickstart::

    from coding_adventures.conduit import Conduit

    app = Conduit()

    @app.get("/hello/<name>")
    def hello(ctx):
        ctx.json({"message": f"Hello {ctx.params['name']}"})

    app.serve(port=3000)

Public API:
    Conduit         — application class (routes, filters, settings)
    HaltException   — raised by response helpers; caught by NativeServer
    NativeServer    — low-level server wrapper (useful for testing)
"""

from .application import Conduit
from .halt_exception import HaltException
from .server import NativeServer

__all__ = ["Conduit", "HaltException", "NativeServer"]
__version__ = "0.1.0"

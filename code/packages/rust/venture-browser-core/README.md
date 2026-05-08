# venture-browser-core

Core browser-shell primitives for Venture.

This crate intentionally contains no platform windowing, networking, parser, or
paint backend code. It covers the small state machines BR01 needs everywhere:
navigation history, scroll offset clamping, and link hit-testing.

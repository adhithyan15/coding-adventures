---
category: Native extensions & FFI
---

# A native FFI exception must pin the build context as well as the shim

An analyzer exception for one C shim is incomplete if it checks only the C and
OCaml wrapper. Dune context can add compiler flags, generated sources, headers,
or link inputs that change what the package builds. Pin the wrapper, shim, and
the complete Dune context, and reject extra native inputs. When a repository
coverage build has a known instrumentation stanza, pin that as a second whole
context bundle rather than accepting arbitrary instrumentation. Keep the
exception bound to exact external declarations and a narrowly declared
capability profile; source hashing identifies the reviewed bytes but does not
prove the C code's behavior or replace the required owner approval.

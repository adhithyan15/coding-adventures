# Linux host for `adj_stdlib_provenance.py verify`.
#
# The verifier requires cgroup v2 delegation (ADJ-STDLIB-COVERAGE 13i) and hard-
# refuses to run without it — `_PosixGuardian` fails unless the platform is
# Linux. That boundary is deliberate, so rather than weaken it, this image
# reproduces the Linux the verifier expects. It also builds the trusted
# parser/audit binaries INSIDE the container: the verifier execs them, and a
# macOS build cannot run here.
#
# PINNED BY DIGEST, not by the floating `1-slim` tag. These binaries are
# *trusted* — the verifier believes their output — so the toolchain that compiles
# them is part of the trust boundary and must not drift silently.
FROM rust:1-slim@sha256:b5b16ce96388d90eb217e162d4729b05c964c5c2388db28f70fbb8a4a66e1d26

# jsonschema is pinned to the SAME version CI installs. Debian's
# python3-jsonschema is 4.19.2 while CI uses 4.26.0, and validator behaviour
# changed between them — a manifest could validate here and fail in CI, which
# would make this harness actively misleading rather than merely different.
RUN apt-get update \
 && apt-get install -y --no-install-recommends python3 python3-pip ca-certificates \
 && pip3 install --break-system-packages --no-cache-dir jsonschema==4.26.0 \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /repo

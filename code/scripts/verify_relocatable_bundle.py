#!/usr/bin/env python3
"""Verify that a built web bundle works when served from a subdirectory.

This exists because `mosaic-emit-react` shipped a `vite.config.ts` with no
`base`, Vite defaulted to `"/"`, and the resulting bundle referenced
`/assets/index-HASH.js`. Served from the root of a domain that resolves. Served
from anywhere else -- a GitHub Pages project site, a staging path, an unzipped
release bundle -- `index.html` returns 200 and its script returns 404, so the
page renders blank. A blank page reads as a working deploy with no content
rather than as a failure, which is why it reached a published release.

Two properties make this check catch that class of bug where the emitter's own
tests could not:

**It serves from a subdirectory, never the root.** Serving from the root passes
with the bug present. The nesting is the entire point, so it is not
configurable.

**It derives what to fetch from `index.html` itself**, rather than from a list
of expected filenames. Vite hashes chunk names, so a hardcoded list either goes
stale or gets loosened until it asserts nothing. Whatever the entry point
actually references is what must resolve.

The emitter's 215 unit tests assert on emitted *text*. That cannot catch a
*missing* key -- an assertion only checks for what it knows to look for -- and
the failure appears one layer down, after a bundler has resolved URLs against
`base`. This is that layer.
"""

from __future__ import annotations

import argparse
import functools
import http.server
import re
import shutil
import socket
import socketserver
import sys
import tempfile
import threading
import urllib.error
import urllib.request
from pathlib import Path

# Where the bundle is mounted under the server root. Deep enough that a single
# accidental `..` in a relative path does not silently land back at the root and
# make a broken bundle look fine.
MOUNT_PREFIX = "nested/deploy/path"

# `src`/`href` on any element. Deliberately not an HTML parser: the input is
# bundler output, not arbitrary markup, and a regex keeps this dependency-free
# in a repo that pins zero-dependency tooling as a rule.
ASSET_REF = re.compile(r'(?:src|href)\s*=\s*["\']([^"\']+)["\']')

# A root-absolute URL: one leading slash, not two (`//host/x` is
# protocol-relative and points off-site, which is a different thing entirely).
ROOT_ABSOLUTE = re.compile(r"^/(?!/)")


class _QuietHandler(http.server.SimpleHTTPRequestHandler):
    """A static handler that does not narrate every request to stderr."""

    def log_message(self, *args: object) -> None:  # noqa: D102 - silence only
        del args


def _free_port() -> int:
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return int(probe.getsockname()[1])


def asset_references(index_html: str) -> list[str]:
    """Every asset URL the entry point references, in document order.

    Data URIs and off-site URLs are dropped: they are not this bundle's files,
    so their resolution says nothing about whether the bundle relocates.
    """

    refs: list[str] = []
    for ref in ASSET_REF.findall(index_html):
        lowered = ref.lower()
        if lowered.startswith(("data:", "http://", "https://", "//", "#", "mailto:")):
            continue
        if ref not in refs:
            refs.append(ref)
    return refs


def root_absolute_references(index_html: str) -> list[str]:
    """The subset of references that only resolve at a domain root."""

    return [ref for ref in asset_references(index_html) if ROOT_ABSOLUTE.match(ref)]


def root_absolute_asset_literals(dist: Path) -> list[str]:
    """Root-absolute string literals in built JS that name this bundle's files.

    The entry-point scan cannot see these. Engram's wasm host held
    `const WASM_URL = "/engram_engine.wasm"` — root-absolute, and invisible to
    any check that reads `index.html`, because script fetches it at runtime.
    Serving the bundle does not catch it either: the file *is* present at the
    bundle-relative path, so `--also` passes while the app still asks for it at
    the wrong URL.

    Scanning minified JS for leading slashes in general would be hopeless — most
    are regexes, dates, or division. This is narrow instead: for each file that
    actually sits at the top of the bundle, look for that exact filename as a
    root-absolute literal. A match names a real file at a URL that only resolves
    at a domain root, which is never what was meant.
    """

    names = sorted(p.name for p in dist.iterdir() if p.is_file() and p.name != "index.html")
    if not names:
        return []

    findings: list[str] = []
    for js in sorted(dist.rglob("*.js")):
        try:
            body = js.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        for name in names:
            for quote in ('"', "'", "`"):
                if f"{quote}/{name}{quote}" in body and name not in findings:
                    findings.append(name)
    return findings


def _fetch_status(url: str) -> int:
    """Fetch ``url`` and return its status, draining the body.

    The drain is not incidental. Closing the connection without reading leaves
    the server mid-`sendall` on anything large -- a 2.7 MB wasm engine, say --
    and it logs a `BrokenPipeError` traceback per request. That noise buries the
    actual verdict, and a check whose output is hard to read is a check people
    stop reading.
    """

    try:
        with urllib.request.urlopen(url, timeout=30) as response:  # noqa: S310
            response.read()
            return int(response.status)
    except urllib.error.HTTPError as error:
        error.read()
        return int(error.code)


def verify(dist: Path, *, extra_paths: tuple[str, ...] = ()) -> list[str]:
    """Serve ``dist`` from a subdirectory and report what fails to resolve.

    Returns a list of human-readable problems; empty means the bundle is
    relocatable. Returning rather than raising lets the caller report every
    failure at once instead of one per run.
    """

    index = dist / "index.html"
    if not index.is_file():
        return [f"no index.html in {dist}"]
    html = index.read_text(encoding="utf-8")

    problems: list[str] = []

    # Checked before serving. A root-absolute reference is a defect even if the
    # file happens to exist at the server root for an unrelated reason, and
    # saying so names the cause rather than the symptom.
    for ref in root_absolute_references(html):
        problems.append(
            f"{ref} is root-absolute, so it only resolves when the app is "
            "served from the root of a domain"
        )

    for name in root_absolute_asset_literals(dist):
        problems.append(
            f'built JS fetches "/{name}" from the domain root; the file is in '
            "the bundle, so serving it succeeds while the app still requests "
            "the wrong URL — use a bundle-relative path"
        )

    # Mount the bundle under a genuinely nested path rather than serving its
    # parent and *describing* the result as nested. The URL the client requests
    # is what relative references resolve against, so if the reported path and
    # the requested path disagree, the check is testing something other than
    # what it claims -- and a report that misstates what it verified is worse
    # than no report.
    #
    # A symlink keeps this O(1) regardless of bundle size; the fallback copy is
    # for filesystems that refuse them.
    with tempfile.TemporaryDirectory() as tmp:
        mount_dir = Path(tmp) / MOUNT_PREFIX
        mount_dir.mkdir(parents=True)
        served = mount_dir / dist.name
        try:
            served.symlink_to(dist.resolve(), target_is_directory=True)
        except (OSError, NotImplementedError):
            shutil.copytree(dist, served)

        mount = f"/{MOUNT_PREFIX}/{dist.name}"
        handler = functools.partial(_QuietHandler, directory=tmp)

        class _Server(socketserver.TCPServer):
            allow_reuse_address = True

        port = _free_port()
        with _Server(("127.0.0.1", port), handler) as httpd:
            thread = threading.Thread(target=httpd.serve_forever, daemon=True)
            thread.start()
            try:
                base = f"http://127.0.0.1:{port}{mount}"
                entry_status = _fetch_status(f"{base}/")
                if entry_status != 200:
                    problems.append(f"index.html at {mount}/ returned {entry_status}")

                for ref in asset_references(html):
                    target = ref[2:] if ref.startswith("./") else ref.lstrip("/")
                    status = _fetch_status(f"{base}/{target}")
                    if status != 200:
                        problems.append(
                            f"{ref} returned {status} when served from {mount}/"
                        )

                for extra in extra_paths:
                    status = _fetch_status(f"{base}/{extra.lstrip('/')}")
                    if status != 200:
                        problems.append(
                            f"{extra} returned {status} when served from {mount}/"
                        )
            finally:
                httpd.shutdown()
                thread.join(timeout=5)

    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "dist", help="The built bundle directory (the one containing index.html)"
    )
    parser.add_argument(
        "--also",
        action="append",
        default=[],
        metavar="PATH",
        help=(
            "An additional bundle-relative path that must resolve. Use for "
            "assets fetched by script rather than referenced from index.html, "
            "such as a wasm engine."
        ),
    )
    args = parser.parse_args(argv)

    problems = verify(Path(args.dist), extra_paths=tuple(args.also))
    if problems:
        print(
            "bundle is not relocatable — it only works at a domain root:",
            file=sys.stderr,
        )
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1

    print(f"{args.dist} resolves correctly when served from a subdirectory")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

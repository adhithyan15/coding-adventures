---
category: Testing & coverage
---

# A path resolver must validate the current node before consuming each component

The OCaml and Swift Unix-history ports initially removed `.` components before
walking the path. That made `/hello/.` resolve to the regular file `/hello`,
while the shared Python model returned `ENOTDIR`: the resolver had already
reached a file before it could process the dot component. A final trailing-slash
check cannot catch this case because the path ends in `.`.

Keep path components until traversal. Before processing every component,
including `.` and `..`, require the current node to be a directory. Add tests
for dot within a directory, dot after a file, dot-dot after a file, dot-dot at
root, and trailing slash behavior; check both file opening and directory listing
where those APIs share the resolver.

The first pinned OCaml build attempt stopped at Dune's `@fmt` alias because
three manually wrapped expressions did not match the package formatter. Run the
package BUILD driver so formatting, compilation, component tests, and fixture
replay all run in the same pinned toolchain.

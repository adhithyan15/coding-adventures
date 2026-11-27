# HOS-002b: affected-only historical Unix fixture routing

## Purpose

Changes to the historical Unix shared replay contract must invalidate the
native package tests that consume it, without scheduling unrelated packages.
This contract applies to the Go build tool's repository-input routing and its
content hashes; it does not change package build commands or simulator APIs.

## Inputs and consumers

An edit, addition, deletion, or rename involving
`code/specs/fixtures/historical-unix-v1/file-read.json` selects these ten
packages: `c/unix-history-simulator`, `cpp/unix-history-simulator`,
`go/unix-history-simulator`, `java/unix-history-simulator`,
`lua/unix-history-simulator`, `perl/unix-history-simulator`,
`python/unix-history-simulator`, `ruby/unix-history-simulator`,
`rust/unix-history-simulator`, and `typescript/unix-history-simulator`.
Their package repository-input hashes include the fixture bytes.

`replay_cli.py` and the shared `build_native.py` driver are read by the C,
C++, Java, and Lua package BUILD recipes, so changes to either select and hash
only those four consumers. The driver runs package component tests and fixture
replay on Linux, macOS, and Windows. Each external adapter source under
`test-adapters/` is read by its matching package recipe: C, C++, Java, or Lua.
The adapter README and similarly named files outside these exact inputs do not
select consumers.

## Selection and hash behavior

Routing is based on changed Git paths, not on whether a changed path currently
exists. Thus additions, edits, deletions, and either side of a rename select
the consumers for the matching old or new path. A rename between matching and
unrelated paths selects consumers for the matching side only. Paths are
repository-relative with `/` separators. Selecting a consumer also selects
its prerequisites through the existing affected-package graph.

The input list for each consumer is finite and exact. Hashing includes only
the declared file contents and uses the build tool's existing missing-input
behavior; no directory-wide glob is used. A fixture-only change does not
trigger a repository-wide build. Language filtering still applies, so a
language-specific invocation cannot select another language's package.

## Verification

Tests pin the package set for the canonical fixture, helper, native driver,
and each adapter; exercise add/edit/delete/rename path selection and
lookalikes; and verify that changing declared content changes the appropriate
package hash while unrelated packages and hash inputs remain unchanged. The
same path-selection contract is checked for each supported platform plan.

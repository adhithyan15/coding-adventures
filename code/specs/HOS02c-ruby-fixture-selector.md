# HOS-002c: exact Ruby Unix fixture selector

## Purpose

The HOS-002 shared fixture must select every package BUILD whose tests read it
outside the package directory. The Ruby simulator is one such consumer, and its
canonical package identity preserves the underscore in its directory name.

This is a narrow correction to Go build-tool repository-input selection. It does
not change the simulator API, the Ruby BUILD command, the shared fixture, or
user-facing CLI capabilities.

## Canonical package and input

The package path is
code/packages/ruby/unix_history_simulator. Go discovery constructs the package
identity from the language directory and package-directory basename, so the
exact identity is ruby/unix_history_simulator.

Its BUILD runs ruby test/test_simulator.rb. That tracked test directly reads
code/specs/fixtures/historical-unix-v1/file-read.json and replays the cases
through the Ruby library in process. It does not read the shared replay helper,
a fixture-local build driver, or an external adapter source.

Therefore the Ruby consumer's only repository input is the exact file
code/specs/fixtures/historical-unix-v1/file-read.json. Adding, editing,
deleting, or renaming that file selects the exact Ruby package identity and
includes the file bytes in that package's repository-input hash.

## Identity and scope

Do not register ruby/unix-history-simulator. That hyphenated identity does not
match the package directory basename and must remain absent. Do not add the
replay helper, native adapter driver, README, or any directory glob to the Ruby
input list. Existing consumers and input sets remain unchanged.

## Verification

The package-routing tests must prove:

- the canonical fixture selects ruby/unix_history_simulator;
- add, edit, delete, and rename changes select that package;
- the hyphenated lookalike identity is not registered;
- unrelated lookalike fixture paths select no Ruby consumer;
- repositoryInputsForPackage("ruby/unix_history_simulator") returns exactly
  the canonical fixture path, while the invalid hyphenated identity returns no
  input list;
- a fixture-content change changes the Ruby package hash and does not change an
  unrelated package hash;
- platform-specific package plans on Linux, macOS, and Windows retain the same
  exact Ruby input and selection behavior.

The existing Ruby package BUILD remains the semantic replay check. Run it with
the repository-pinned Ruby runtime where available.

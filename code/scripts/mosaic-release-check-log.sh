#!/usr/bin/env bash
# mosaic-release-check-log.sh -- show a release launch log, and fail if the
# app said its bundled runtime did not load.
#
#   usage: mosaic-release-check-log.sh LABEL LOG
#
# The Linux release lanes launch a payload with `timeout`, require exit 124
# ("still running when I killed it"), and then have to ask one more question
# about the log: did the host start WITHOUT its Rust runtime? A Qt or Flutter
# host can keep its window up after printing
#
#     Mosaic Rust runtime unavailable
#
# so exit 124 alone would pass a payload that ships no working runtime. See
# code/specs/mosaic-release-state-gate-v1.md, "Launching a payload".
#
# Every lane used to carry its own copy of that check, and the copies drifted:
#
#   copy                                   what it actually did
#   -------------------------------------  ---------------------------------
#   ! grep -Fq "...unavailable" "$log"     nothing. `set -e` ignores a
#   (TaskApp Qt, Flutter, Compose)         command inverted with `!`, so a
#                                          log that said it still passed
#   cat "$log"                             printed the app's output where
#                                          GitHub reads `::command::` lines
#   if grep -F "...unavailable" "$log"     right exit, but echoed the matched
#   (Engram Compose, Flutter)              line outside `::stop-commands::`
#
# This script is the one copy. It
#
#   1. prints LOG inside `::group::LABEL`, between `::stop-commands::<token>`
#      and `::<token>::`, so a line the app printed such as `::add-mask::` or
#      `::endgroup::` is shown as text, not obeyed. The token is random and is
#      made after the app has exited, so the app cannot have printed it;
#   2. fails if LOG contains the runtime-unavailable report, without echoing
#      the matching line (`grep -q`).
#
# Exit status:
#
#   0   LOG is readable and does not report "Mosaic Rust runtime unavailable"
#   1   usage error, LOG missing or unreadable, or LOG reports it
#
# The macOS launcher (mosaic-release-launch.sh) applies the same rule to the
# log it captures; this script is for lanes that capture their own.

set -uo pipefail

RUNTIME_UNAVAILABLE="Mosaic Rust runtime unavailable"

if [[ $# -ne 2 ]]; then
  echo "usage: mosaic-release-check-log.sh LABEL LOG" >&2
  exit 1
fi
label="$1"
log="$2"

# A missing log means the launch never wrote one, so there is nothing to
# vouch for. Passing here would turn "no evidence" into "no problem".
if [[ ! -f "$log" || ! -r "$log" ]]; then
  echo "::error::$label: no readable launch log at $log" >&2
  exit 1
fi

# 128 random bits from the kernel; $RANDOM (15 bits) only if there is no
# /dev/urandom. Either way it did not exist while the app was writing LOG.
token="$(od -An -N16 -tx1 /dev/urandom 2>/dev/null | tr -d ' \n')"
if [[ -z "$token" ]]; then
  token="$$-$RANDOM$RANDOM$RANDOM"
fi
token="mosaic-log-$token"

echo "::group::$label"
echo "::stop-commands::$token"
cat "$log"
# A log whose last line has no newline would glue the resume command onto
# it, and GitHub only reads a command at the start of a line.
if [[ -s "$log" && -n "$(tail -c 1 "$log")" ]]; then
  echo
fi
echo "::$token::"
echo "::endgroup::"

if grep -Fq "$RUNTIME_UNAVAILABLE" "$log"; then
  echo "::error::$label: the app ran, but could not load its bundled runtime" >&2
  exit 1
fi
exit 0

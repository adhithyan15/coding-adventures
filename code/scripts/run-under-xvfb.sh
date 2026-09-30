#!/usr/bin/env bash
# Run a command on a virtual X display and exit with THE COMMAND'S status.
#
#   run-under-xvfb.sh COMMAND [ARGS...]
#
# e.g. the launch checks that expect an app to be killed by `timeout`:
#
#   run-under-xvfb.sh timeout 10s "$launcher"    # exits 124 when it ran 10s
#
# Why not plain `xvfb-run -a COMMAND`: after the command exits, xvfb-run stops
# the X server and deletes its temporary directory, and when that deletion
# fails it prints "xvfb-run: error: problem while cleaning up temporary
# directory" and exits non-zero IN PLACE OF the command's status. A launch
# check that wanted 124 ("still running when the timeout fired") then fails
# although the app ran exactly as it should (seen on the Compose Linux release
# payload, PR #16255). The command's own status is what every caller asks
# about, so this records it inside the display session and returns it:
#
#   command ran | xvfb-run exit | this script exits
#   ------------+---------------+-----------------------------------------
#   yes, N      | N             | N
#   yes, N      | cleanup error | N (and says xvfb-run disagreed)
#   no          | M             | M, or 1 if M is 0 (the display never came up)
set -uo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 COMMAND [ARGS...]" >&2
  exit 2
fi

status_file="$(mktemp)"
trap 'rm -f -- "$status_file"' EXIT

# `sh -c` receives the status file as $0 and the command as "$@", so nothing
# the caller passes is ever parsed as shell text; it exits with the command's
# status too, so xvfb-run agrees with it whenever cleanup succeeds.
# shellcheck disable=SC2016
xvfb-run -a sh -c '"$@"; s=$?; echo "$s" > "$0"; exit "$s"' "$status_file" "$@"
xvfb_status=$?

if [[ -s "$status_file" ]]; then
  status="$(cat -- "$status_file")"
  if [[ "$xvfb_status" -ne "$status" ]]; then
    echo "run-under-xvfb: xvfb-run exited $xvfb_status after the command exited $status; reporting the command's status" >&2
  fi
  exit "$status"
fi

echo "run-under-xvfb: the command never ran (xvfb-run exited $xvfb_status)" >&2
if [[ "$xvfb_status" -eq 0 ]]; then
  exit 1
fi
exit "$xvfb_status"

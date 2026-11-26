#!/usr/bin/env bash
set -euo pipefail

out_dir="${RUNNER_TEMP:?RUNNER_TEMP must be set}/kiln-ptrace-diagnostic"
shim_dir="$out_dir/shim"
mkdir -m 700 -p "$shim_dir"
test_log="$out_dir/go-test.log"
outer_trace="$out_dir/outer-strace.log"
metadata="$out_dir/metadata.txt"
: > "$test_log"
: > "$outer_trace"
: > "$metadata"

head_sha="$(git rev-parse HEAD)"
if [[ ! "${EXPECTED_SHA:-}" =~ ^[0-9a-f]{40}$ ]]; then
  echo "expected_sha must be the reviewed full lowercase commit SHA" >&2
  exit 2
fi
if [[ "$head_sha" != "$EXPECTED_SHA" ]]; then
  echo "checked-out HEAD $head_sha does not match expected_sha $EXPECTED_SHA" >&2
  exit 2
fi

if ! real_strace="$(command -v strace)"; then
  echo "strace executable was not found on PATH" >&2
  exit 2
fi
if [[ ! -x "$real_strace" ]]; then
  echo "strace executable not found: $real_strace" >&2
  exit 2
fi
case "$real_strace" in
  /*) ;;
  *) echo "strace path is not absolute: $real_strace" >&2; exit 2 ;;
esac
if [[ "$real_strace" != "/usr/bin/strace" ]]; then
  echo "diagnostic shim requires the runner's /usr/bin/strace; found $real_strace" >&2
  exit 2
fi

{
  printf 'kernel='; uname -a
  printf 'go='; go version
  printf 'strace_path=%s\n' "$real_strace"
  strace -V 2>&1 | head -1
  printf 'head=%s\n' "$head_sha"
  printf 'expected_sha=%s\n' "$EXPECTED_SHA"
  printf 'command=go test -timeout 10m -count=5 -run ^TestSandboxStraceDyingThreads$ -v .\n'
} | tee "$metadata"

# runStracedImpl uses LookPath in the test process, then gives the child a
# minimal PATH. The shim is therefore resolved before that environment reset.
# It traces only the inner strace process: deliberately no outer -f, so the
# outer tracer never attaches to the inner tracer's target.
cat > "$shim_dir/strace" <<'SHIM'
#!/usr/bin/env bash
set -euo pipefail
shim_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
real_strace="/usr/bin/strace"
exec "$real_strace" -e trace=ptrace,wait4,waitid -ttt -T -s 256 -A \
  -o "$shim_dir/outer-strace.log" -- "$real_strace" "$@"
SHIM
chmod 700 "$shim_dir/strace"

set +e
PATH="$shim_dir:$PATH" \
KILN_REQUIRE_SANDBOX=1 \
KILN_TRACE_DIFF=1 \
go test -timeout 10m -count=5 -run '^TestSandboxStraceDyingThreads$' -v . \
  > "$test_log" 2>&1
test_status=$?
set -e

cat "$test_log"
printf 'go_test_exit=%s\n' "$test_status"
if [[ ! -f "$outer_trace" ]]; then
  echo "outer syscall trace was not created" >&2
  exit 2
fi
printf 'go_test_log_bytes=%s\n' "$(wc -c < "$test_log")"
printf 'outer_trace_bytes=%s\n' "$(wc -c < "$outer_trace")"
{
  printf 'go_test_exit=%s\n' "$test_status"
  printf 'go_test_log_bytes=%s\n' "$(wc -c < "$test_log")"
  printf 'outer_trace_bytes=%s\n' "$(wc -c < "$outer_trace")"
  sha256sum "$test_log" "$outer_trace"
} | tee -a "$metadata"
exit "$test_status"

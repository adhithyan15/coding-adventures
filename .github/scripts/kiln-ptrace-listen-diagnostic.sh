#!/usr/bin/env bash
set -euo pipefail

out_dir="${RUNNER_TEMP:?RUNNER_TEMP must be set}/kiln-ptrace-diagnostic-${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-1}"
shim_dir="$out_dir/shim"
umask 077
mkdir -m 700 -p "$shim_dir"
test_log="$out_dir/go-test.log"
outer_trace="$shim_dir/outer-strace.log"
test_truncated="$out_dir/go-test.truncated"
metadata="$out_dir/metadata.txt"
: > "$test_log"
: > "$metadata"
test_log_cap_bytes=8388608
trace_cap_bytes=33554432

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(cd -- "$script_dir/../.." && pwd)"
cd "$repo_root"

head_sha="$(git rev-parse HEAD)"
{
  printf 'head=%s\n' "$head_sha"
  printf 'expected_sha=%s\n' "${EXPECTED_SHA:-}"
  printf 'checkout_root=%s\n' "$repo_root"
  printf 'go_test_exit=not_run\n'
} | tee -a "$metadata"
if [[ ! "${EXPECTED_SHA:-}" =~ ^[0-9a-f]{40}$ ]]; then
  echo "expected_sha must be the reviewed full lowercase commit SHA" >&2
  printf 'sha_guard=invalid_expected_sha\n' | tee -a "$metadata"
  exit 2
fi
if [[ "$head_sha" != "$EXPECTED_SHA" ]]; then
  echo "checked-out HEAD $head_sha does not match expected_sha $EXPECTED_SHA" >&2
  printf 'sha_guard=head_mismatch\n' | tee -a "$metadata"
  exit 2
fi
printf 'sha_guard=pass\n' | tee -a "$metadata"

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
if [[ ! -x /usr/bin/prlimit ]]; then
  echo "required util-linux prlimit executable is missing" >&2
  exit 2
fi

module_dir="$repo_root/code/programs/go/kiln"
cd "$module_dir"

{
  printf 'kernel='; uname -a
  printf 'go='; go version
  printf 'python='; python3 --version
  printf 'strace_path=%s\n' "$real_strace"
  "$real_strace" -V 2>&1 | head -1
  printf 'cwd=%s\n' "$PWD"
  printf 'run_env=KILN_REQUIRE_SANDBOX=1 KILN_TRACE_DIFF=1\n'
  printf 'capture_helper=.github/scripts/capture-bounded.py\n'
  printf 'outer_filter=ptrace,wait4,waitid; no_follow_forks\n'
  printf 'file_size_limit_tool=/usr/bin/prlimit\n'
  printf 'test_log_cap_bytes=%s\n' "$test_log_cap_bytes"
  printf 'trace_cap_bytes=%s\n' "$trace_cap_bytes"
  printf 'command=go test -timeout 10m -count=5 -run ^TestSandboxStraceDyingThreads$ -v .\n'
} | tee -a "$metadata"

# runStracedImpl uses LookPath in the test process, then gives the child a
# minimal PATH. The shim is therefore resolved before that environment reset.
# It traces only the inner strace process: deliberately no outer -f, so the
# outer tracer never attaches to the inner tracer's target.
cat > "$shim_dir/strace" <<'SHIM'
#!/usr/bin/env bash
set -euo pipefail
shim_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
real_strace="/usr/bin/strace"
trace_cap_bytes=33554432
exec /usr/bin/prlimit --fsize="${trace_cap_bytes}:${trace_cap_bytes}" -- \
  "$real_strace" -e trace=ptrace,wait4,waitid -ttt -T -s 256 -A \
  -o "$shim_dir/outer-strace.log" -- "$real_strace" "$@"
SHIM
chmod 700 "$shim_dir/strace"

set +e
set -o pipefail
PATH="$shim_dir:$PATH" \
KILN_REQUIRE_SANDBOX=1 \
KILN_TRACE_DIFF=1 \
go test -timeout 10m -count=5 -run '^TestSandboxStraceDyingThreads$' -v . \
  2>&1 | python3 "$repo_root/.github/scripts/capture-bounded.py" \
    --max-bytes "$test_log_cap_bytes" --output "$test_log" \
    --truncated-marker "$test_truncated"
pipeline_status=("${PIPESTATUS[@]}")
test_status="${pipeline_status[0]}"
capture_status="${pipeline_status[1]}"
set -e

cat "$test_log"
printf 'go_test_exit=%s\n' "$test_status"
test_log_bytes=0
if [[ -f "$test_log" ]]; then
  test_log_bytes="$(wc -c < "$test_log")"
fi
outer_trace_bytes=0
if [[ -f "$outer_trace" ]]; then
  outer_trace_bytes="$(wc -c < "$outer_trace")"
fi
test_log_truncated=0
if [[ -f "$test_truncated" ]]; then test_log_truncated=1; fi
run_count="$(grep -cE '^=== RUN[[:space:]]+TestSandboxStraceDyingThreads$' "$test_log" || true)"
skip_count="$(grep -cE '^--- SKIP: TestSandboxStraceDyingThreads([[:space:](]|$)' "$test_log" || true)"
trace_cap_reached=0
if (( outer_trace_bytes >= trace_cap_bytes )); then trace_cap_reached=1; fi
trace_nonempty=0
if (( outer_trace_bytes > 0 )); then trace_nonempty=1; fi
trace_has_ptrace=0
if [[ -f "$outer_trace" ]] && grep -q 'ptrace(' "$outer_trace"; then
  trace_has_ptrace=1
fi

diagnostic_status="$test_status"
if [[ "$test_status" -eq 0 ]] && {
  [[ "$capture_status" -ne 0 ]] ||
  [[ "$test_log_truncated" -ne 0 ]] ||
  [[ "$run_count" -ne 5 ]] ||
  [[ "$skip_count" -ne 0 ]] ||
  [[ "$trace_cap_reached" -ne 0 ]] ||
  [[ "$trace_nonempty" -ne 1 ]] ||
  [[ "$trace_has_ptrace" -ne 1 ]]
}; then
  diagnostic_status=3
fi

{
  printf 'go_test_exit=%s\n' "$test_status"
  printf 'capture_exit=%s\n' "$capture_status"
  printf 'go_test_log_bytes=%s\n' "$test_log_bytes"
  printf 'go_test_log_truncated=%s\n' "$test_log_truncated"
  printf 'test_run_markers=%s\n' "$run_count"
  printf 'test_skip_markers=%s\n' "$skip_count"
  printf 'outer_trace_bytes=%s\n' "$outer_trace_bytes"
  printf 'outer_trace_nonempty=%s\n' "$trace_nonempty"
  printf 'outer_trace_has_ptrace=%s\n' "$trace_has_ptrace"
  printf 'outer_trace_cap_reached=%s\n' "$trace_cap_reached"
  printf 'diagnostic_exit=%s\n' "$diagnostic_status"
  [[ ! -f "$test_log" ]] || sha256sum "$test_log"
  [[ ! -f "$test_truncated" ]] || sha256sum "$test_truncated"
  [[ ! -f "$outer_trace" ]] || sha256sum "$outer_trace"
} | tee -a "$metadata"
exit "$diagnostic_status"

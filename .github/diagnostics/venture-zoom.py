"""One exact native test, with bounded snapshots of its own disposable outputs.

Successful tests remove their nonce directory. A read-only observer therefore
mirrors the allowlisted evidence while the original test runs. Its exit status
remains the job's exit status; missing evidence never becomes a passing test.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import threading

ART = Path(os.environ['DIAGNOSTIC_ARTIFACT_ROOT'])
TMP = Path(os.environ['TMPDIR'])
ART.mkdir(parents=True, exist_ok=False)
TMP.mkdir(parents=True, exist_ok=False)
LIMITS = {'swiftui-app.log': 5 * 1024 * 1024,
          'swiftui-ready.json': 128 * 1024,
          'swiftui-interaction.json': 128 * 1024,
          'swiftui-zoom-failure.png': 1500000}
stop = threading.Event()
records = {}
seen = set()
observer_errors = []
root_fd = os.open(TMP, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)


def snapshot():
    """Use no-follow directory-relative opens; never traverse arbitrary output."""
    names = sorted(os.listdir(root_fd))
    for name in names:
        if not re.fullmatch(r'venture-swiftui-launch-[0-9]+-[0-9]+', name):
            continue
        if name not in seen and len(seen) >= 8:
            records['directory_limit'] = {'status': 'directory_limit_exceeded'}
            continue
        seen.add(name)
        try:
            directory = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                                dir_fd=root_fd)
        except OSError as error:
            records[name] = {'status': 'directory_unavailable', 'errno': error.errno}
            continue
        try:
            destination = ART / name
            destination.mkdir(exist_ok=True)
            for filename, limit in LIMITS.items():
                key = name + '/' + filename
                try:
                    fd = os.open(filename, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK,
                                 dir_fd=directory)
                    with os.fdopen(fd, 'rb') as source:
                        info = os.fstat(source.fileno())
                        if not stat.S_ISREG(info.st_mode):
                            records[key] = {'status': 'rejected_nonregular'}
                            continue
                        data = source.read(limit + 1)
                    truncated = len(data) > limit or info.st_size > limit
                    data = data[:limit]
                    temporary = destination / (filename + '.tmp')
                    temporary.write_bytes(data)
                    temporary.replace(destination / filename)
                    records[key] = {'status': 'truncated' if truncated else 'captured_snapshot',
                                    'bytes': len(data), 'observed_size': info.st_size,
                                    'sha256': hashlib.sha256(data).hexdigest()}
                except FileNotFoundError:
                    records.setdefault(key, {'status': 'not_observed'})
                except OSError as error:
                    records[key] = {'status': 'unavailable', 'errno': error.errno}
        finally:
            os.close(directory)


def observe():
    while not stop.is_set():
        try:
            snapshot()
        except Exception as error:
            if len(observer_errors) < 8:
                observer_errors.append(type(error).__name__)
        stop.wait(0.1)


command = ['cargo', 'test', '-p', 'venture-browser-macos', '--test',
           'swiftui_project_launch',
           'package_owned_swiftui_project_launches_renders_and_interacts',
           '--', '--exact', '--nocapture']
thread = threading.Thread(target=observe, daemon=True)
thread.start()
exit_code = 125
total = 0
kept = 0
output_limit = 16 * 1024 * 1024
console_limit = 8 * 1024 * 1024
console_kept = 0
try:
    with (ART / 'cargo-output.log').open('wb') as output:
        child_env = dict(os.environ)
        child_env.pop('DIAGNOSTIC_ARTIFACT_ROOT', None)
        child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                 env=child_env)
        while True:
            chunk = child.stdout.read(65536)
            if not chunk:
                break
            total += len(chunk)
            console_chunk = chunk[:max(0, console_limit - console_kept)]
            if console_chunk:
                sys.stdout.buffer.write(console_chunk)
                sys.stdout.buffer.flush()
                console_kept += len(console_chunk)
            retained = chunk[:max(0, output_limit - kept)]
            output.write(retained)
            kept += len(retained)
        exit_code = child.wait()
finally:
    stop.set()
    # Join before reading shared records or closing its directory descriptor.
    # Reads are nonblocking regular files with byte caps; the job's finite
    # 30-minute outer deadline also covers a stalled host filesystem.
    thread.join()
    try:
        snapshot()
    except Exception as error:
        observer_errors.append(type(error).__name__)
    os.close(root_fd)
    essential = list(LIMITS)[:3]
    complete_directories = []
    for name in sorted(seen):
        if all(records.get(name + '/' + filename, {}).get('status') == 'captured_snapshot'
               for filename in essential):
            try:
                for filename in essential[1:]:
                    json.loads((ART / name / filename).read_text(encoding='utf-8'))
                lines = (ART / name / essential[0]).read_text(encoding='utf-8').splitlines()
                telemetry = []
                for line in lines:
                    try:
                        value = json.loads(line)
                    except ValueError:
                        continue
                    if isinstance(value, dict):
                        telemetry.append(value.get('event'))
                if 'zoom_pre_click' in telemetry and 'zoom_poll' in telemetry:
                    complete_directories.append(name)
            except (OSError, ValueError):
                pass
    capture_valid = (len(complete_directories) == 1 and not observer_errors
                     and not thread.is_alive() and total <= kept
                     and not any(value.get('status') in {'truncated', 'directory_limit_exceeded'}
                                 for value in records.values()))
    diagnostic_exit = exit_code if exit_code != 0 else (0 if capture_valid else 2)
    summary = {'command': command, 'cargo_exit_code': exit_code,
               'diagnostic_exit_code': diagnostic_exit,
               'capture_valid': capture_valid,
               'complete_essential_directories': complete_directories,
               'source_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
               'source_tree': subprocess.check_output(['git', 'rev-parse', 'HEAD^{tree}'], text=True).strip(),
               'workflow_head': os.environ.get('GITHUB_SHA'),
               'run_id': os.environ.get('GITHUB_RUN_ID'),
               'cargo_output_total_bytes': total, 'cargo_output_retained_bytes': kept,
               'cargo_output_truncated': total > kept,
               'console_output_retained_bytes': console_kept,
               'console_output_truncated': total > console_kept,
               'observer_still_running': thread.is_alive(),
               'observer_errors': observer_errors[:8], 'files': records,
               'scope': 'Snapshots only; a success may remove files before the last poll. '
                        'No claim that observed snapshots are complete final files.'}
    (ART / 'collection.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'cargo_exit_code': exit_code, 'diagnostic_exit_code': diagnostic_exit,
                  'capture_valid': capture_valid, 'console_truncated': total > console_kept}))
raise SystemExit(diagnostic_exit if diagnostic_exit >= 0 else 128 - diagnostic_exit)

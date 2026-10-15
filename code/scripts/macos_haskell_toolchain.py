"""Build a Cabal-only macOS toolchain overlay and prove real link/run support.

This bootstrap never changes the runner's developer tree or global SDK. Cabal
gets an isolated copy of its existing config only after all real probes pass.
Portable mocked tests cover control flow; the hosted probes establish linking.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import shlex
import sys
import subprocess
from pathlib import Path, PurePosixPath
from typing import Callable


def locate_config(env: dict[str, str], home: Path) -> Path:
    """Mirror Cabal 3.10 discovery, including its documented legacy fallback."""
    if env.get("CABAL_CONFIG"):
        return Path(env["CABAL_CONFIG"])
    if env.get("CABAL_DIR"):
        return Path(env["CABAL_DIR"]) / "config"
    xdg = Path(env.get("XDG_CONFIG_HOME", str(home / ".config"))) / "cabal/config"
    legacy = home / ".cabal"
    return legacy / "config" if legacy.is_dir() and not xdg.exists() else xdg


def checked_macos_path(value: str) -> str:
    if not PurePosixPath(value).is_absolute() or any(ord(c) < 32 or ord(c) == 127 for c in value):
        raise ValueError("toolchain paths must be absolute macOS paths without control characters")
    return value


def phase_options(clang: str, sdk: str, ld: str) -> list[str]:
    checked_macos_path(clang)
    checked_macos_path(sdk)
    checked_macos_path(ld)
    return ["-pgmc", clang, "-pgma", clang, "-pgml", clang,
            "-optc-isysroot", "-optc" + sdk, "-opta-isysroot", "-opta" + sdk,
            "-optl-isysroot", "-optl" + sdk, "-optl-fuse-ld=" + ld]


def unquoted_config_text(line: str) -> str:
    """Expose structure only, without treating quoted arguments as sections."""
    output = []
    quote = None
    escaped = False
    for index, character in enumerate(line):
        if quote is not None:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == quote:
                quote = None
            output.append(" ")
        elif character in "\"'":
            quote = character
            output.append(" ")
        elif line[index:index + 2] == "--" and (index == 0 or line[index - 1].isspace()):
            break
        else:
            output.append(character)
    return "".join(output)


def overlay_config(source: str, clang: str, ld: str, sdk: str,
                   *, hsc_wrapper: Path | None = None,
                   phase_override: tuple[str, Path] | None = None) -> str:
    """Preserve every unmanaged line; refuse competing active tool overrides.

    Fresh hosted configs normally contain commented defaults. A preexisting
    active override deserves review, rather than silent replacement/precedence.
    Paths use Cabal's quoted argument tokens, never a shell command string.
    """
    checked_macos_path(ld)
    options = phase_options(clang, sdk, ld)
    if phase_override is not None:
        phase, missing = phase_override
        if phase not in {"-pgmc", "-pgma", "-pgml"} or not missing.is_absolute():
            raise ValueError("invalid internal phase refusal")
        options[options.index(phase) + 1] = str(missing)
    if hsc_wrapper is not None and (not hsc_wrapper.is_absolute()
            or any(ord(c) < 32 for c in str(hsc_wrapper))):
        raise ValueError("invalid generated hsc2hs wrapper path")
    fields = {
        "program-locations": {"gcc-location": [clang], "ld-location": [ld],
                              "hsc2hs-location": [] if hsc_wrapper is None else [str(hsc_wrapper)]},
        "program-default-options": {
            "ghc-options": options,
            "hsc2hs-options": ["--cc=" + clang, "--ld=" + clang,
                               "--cflag=-isysroot", "--cflag=" + sdk,
                               "--lflag=-isysroot", "--lflag=" + sdk, "--lflag=-fuse-ld=" + ld],
        },
    }
    lines = source.splitlines(keepends=True)
    # Cabal normalizes names. Admit only the line-oriented managed sections
    # implemented here; alternate brace/indent forms must never evade refusal.
    names = re.compile(r"\b(program-locations|program-default-options)\b", re.I)
    for line in lines:
        active = unquoted_config_text(line).strip()
        if names.search(active) and (line[0].isspace() or active.casefold() not in fields):
            raise ValueError("unsupported managed Cabal section syntax")
    for section, managed in fields.items():
        starts = [i for i, line in enumerate(lines)
                  if line and not line[0].isspace() and unquoted_config_text(line).strip().casefold() == section]
        if len(starts) > 1:
            raise ValueError("duplicate active Cabal section: " + section)
        if starts:
            start = starts[0]
            end = next((i for i in range(start + 1, len(lines))
                        if lines[i].strip() and not lines[i][0].isspace()
                        and not lines[i].lstrip().startswith("--")), len(lines))
            for line in lines[start + 1:end]:
                if line.lstrip().startswith("--"):
                    continue
                active = unquoted_config_text(line)
                if any(c in active for c in "{};"):
                    raise ValueError("unsupported managed Cabal section structure")
                match = re.match(r"\s+([A-Za-z0-9-]+)\s*:", line)
                if match and match[1].casefold() in managed:
                    raise ValueError("existing managed Cabal field: " + match[1])
        else:
            if lines and not lines[-1].endswith("\n"):
                lines[-1] += "\n"
            lines.append(section + "\n")
            end = len(lines)
        additions = ["  " + field + ": " + " ".join(json.dumps(v) for v in values) + "\n"
                     for field, values in managed.items() if values]
        lines[end:end] = additions
    return "".join(lines)


def write_hsc_driver(output: Path, real_hsc2hs: str, clang: str, ld: str, sdk: str) -> tuple[Path, list[str]]:
    """Append forced hsc2hs arguments after Cabal's own choices, and trace them."""
    forced = ["--cc=" + clang, "--ld=" + clang,
              "--cflag=-isysroot", "--cflag=" + sdk,
              "--lflag=-isysroot", "--lflag=" + sdk, "--lflag=-fuse-ld=" + ld,
              "--cflag=-DCI_TOOLCHAIN_HSC=1"]
    driver = output / "hsc2hs-driver.py"
    (output / "hsc2hs-driver.json").write_text(json.dumps({"real_hsc2hs": real_hsc2hs, "forced": forced}), encoding="utf-8")
    driver.write_text("import json, os, sys, subprocess, signal\nfrom pathlib import Path\np=Path(__file__)\nc=json.loads(p.with_suffix('.json').read_text(encoding='utf-8'))\na=[c['real_hsc2hs'],*sys.argv[1:],*c['forced']]\nwith (p.parent/'hsc2hs-invocations.jsonl').open('a',encoding='utf-8') as f:\n f.write(json.dumps({'argv':a})+'\\n')\nr=subprocess.run(a,check=False)\nif r.returncode<0:\n sig=-r.returncode\n if sig not in {getattr(signal,'SIGKILL',None),getattr(signal,'SIGSTOP',None)}: signal.signal(sig,signal.SIG_DFL)\n os.kill(os.getpid(),sig)\nraise SystemExit(r.returncode)\n", encoding="utf-8")
    wrapper = output / "hsc2hs"
    wrapper.write_text("#!/bin/sh\nexec " + shlex.quote(sys.executable) + " " + shlex.quote(str(driver)) + ' "$@"\n', encoding="utf-8")
    wrapper.chmod(0o700)
    return wrapper, forced


def verify_hsc_route(output: Path, real_hsc2hs: str, forced: list[str]) -> int:
    trace = output / "hsc2hs-invocations.jsonl"
    if not trace.is_file():
        raise RuntimeError("Cabal did not invoke the selected hsc2hs wrapper")
    events = [json.loads(line)["argv"] for line in trace.read_text(encoding="utf-8").splitlines()]
    source_events = [argv for argv in events if any(Path(arg).name == "Main.hsc" for arg in argv[1:])]
    if not source_events or any(argv[0] != real_hsc2hs or argv[-len(forced):] != forced for argv in source_events):
        raise RuntimeError("Cabal hsc2hs effective driver arguments do not match the selected route")
    return len(source_events)


def bootstrap(output: Path, github_env: Path, env: dict[str, str],
              *, runner: Callable = subprocess.run) -> dict:
    """Fail closed at every step; publish CABAL_CONFIG only after real probes."""
    output = output.resolve()
    output.mkdir(parents=True, mode=0o700, exist_ok=False)
    receipt = {"success": False, "commands": [], "local_miri": "DEFERRED/PENDING"}
    receipt_path = output / "receipt.json"
    def save() -> None:
        receipt_path.write_text(json.dumps(receipt, indent=2), encoding="utf-8")
    def run(label: str, command: list[str], *, cwd: Path = output,
            child_env: dict[str, str] | None = None,
            expected_refusal: Path | None = None) -> str:
        result = runner(command, cwd=cwd, env=dict(env if child_env is None else child_env),
                        capture_output=True, text=True, check=False)
        log = output / (label + ".log")
        log.write_text(result.stdout + result.stderr, encoding="utf-8")
        receipt["commands"].append({"label": label, "argv": command,
                                    "exit": result.returncode, "log": str(log),
                                    "sha256": hashlib.sha256(log.read_bytes()).hexdigest()})
        save()
        if expected_refusal is not None:
            text = result.stdout + result.stderr
            if result.returncode == 0 or ("could not execute: " + str(expected_refusal)) not in text:
                raise RuntimeError(f"{label} did not prove the configured phase refusal; see {log}")
            receipt["commands"][-1]["expected_refusal"] = str(expected_refusal)
            save()
        elif result.returncode:
            raise RuntimeError(f"{label} failed ({result.returncode}); see {log}")
        return result.stdout
    try:
        original = locate_config(env, Path.home())
        original_bytes = original.read_bytes()
        receipt["original_config"] = str(original)
        receipt["original_config_sha256"] = hashlib.sha256(original_bytes).hexdigest()
        clang = checked_macos_path(run("discover-clang", ["/usr/bin/xcrun", "--sdk", "macosx", "--find", "clang"]).strip())
        ld = checked_macos_path(run("discover-ld", ["/usr/bin/xcrun", "--sdk", "macosx", "--find", "ld"]).strip())
        sdk = checked_macos_path(run("discover-sdk", ["/usr/bin/xcrun", "--sdk", "macosx", "--show-sdk-path"]).strip())
        ghc, cabal, hsc2hs = (shutil.which(name, path=env.get("PATH")) for name in ("ghc", "cabal", "hsc2hs"))
        if not ghc or not cabal or not hsc2hs:
            raise RuntimeError("GHC, hsc2hs and Cabal must be installed before macOS bootstrap")
        receipt.update(clang=clang, ld=ld, sdk=sdk, ghc=ghc, cabal=cabal, hsc2hs=hsc2hs)
        wrapper, forced_hsc = write_hsc_driver(output, hsc2hs, clang, ld, sdk)
        receipt["hsc_wrapper_sha256"] = hashlib.sha256(wrapper.read_bytes()).hexdigest()
        config = output / "cabal.config"
        config.write_text(overlay_config(original_bytes.decode("utf-8"), clang, ld, sdk, hsc_wrapper=wrapper), encoding="utf-8")
        receipt["overlay_config_sha256"] = hashlib.sha256(config.read_bytes()).hexdigest()
        run("clang-version", [clang, "--version"])
        if run("ghc-version", [ghc, "--numeric-version"]).strip() != "9.4.8":
            raise RuntimeError("bootstrap requires the reviewed GHC9.4.8")
        if not run("cabal-version", [cabal, "--numeric-version"]).strip().startswith("3.10."):
            raise RuntimeError("bootstrap requires the reviewed Cabal3.10 series")
        run("ghc-settings", [ghc, "--info"])
        c_main = output / "probe.c"
        c_main.write_text('#include <stdio.h>\nint main(void) { puts("42"); return 0; }\n', encoding="utf-8")
        c_binary = output / "c-probe"
        run("c-build", [clang, "-isysroot", sdk, "-fuse-ld=" + ld, str(c_main), "-o", str(c_binary)])
        if run("c-run", [str(c_binary)]).strip() != "42":
            raise RuntimeError("C runtime output mismatch")
        package = output / "cabal-probe"
        package.mkdir()
        main_hs = output / "Main.hs"
        main_hs.write_text('{-# LANGUAGE ForeignFunctionInterface #-}\nmodule Main where\nimport Foreign.C.Types\nforeign import ccall "ci_answer" answer :: IO CInt\nmain :: IO ()\nmain = answer >>= print . (+ (1 :: Int)) . fromIntegral\n', encoding="utf-8")
        answer_c = package / "answer.c"
        answer_c.write_text("int ci_answer(void) { return 41; }\n", encoding="utf-8")
        ghc_dir = output / "ghc-probe"
        ghc_dir.mkdir()
        ghc_binary = ghc_dir / "probe"
        run("ghc-build", [ghc, *phase_options(clang, sdk, ld), str(main_hs), str(answer_c),
                          "-outputdir", str(ghc_dir), "-o", str(ghc_binary), "-v"], cwd=ghc_dir)
        if run("ghc-run", [str(ghc_binary)]).strip() != "42":
            raise RuntimeError("GHC foreign-C runtime output mismatch")
        (package / "ci-toolchain-probe.cabal").write_text("cabal-version: 2.4\nname: ci-toolchain-probe\nversion: 0.0.0\nbuild-type: Simple\nexecutable ci-toolchain-probe\n  main-is: Main.hs\n  c-sources: answer.c\n  build-depends: base >= 4.17 && < 4.18\n  default-language: Haskell2010\n", encoding="utf-8")
        (package / "cabal.project").write_text("packages: .\n", encoding="utf-8")
        (package / "Main.hsc").write_text('{-# LANGUAGE ForeignFunctionInterface #-}\nmodule Main where\nimport Foreign.C.Types\nforeign import ccall "ci_answer" answer :: IO CInt\nprobe_marker :: Int\nprobe_marker = #{const CI_TOOLCHAIN_HSC}\nmain :: IO ()\nmain = answer >>= print . (+ probe_marker) . fromIntegral\n', encoding="utf-8")
        for phase in ["-pgmc", "-pgma", "-pgml"]:
            name = phase.removeprefix("-")
            missing = output / ("missing-" + name)
            refused = output / ("refuse-" + name + ".config")
            refused.write_text(overlay_config(original_bytes.decode("utf-8"), clang, ld, sdk,
                hsc_wrapper=wrapper, phase_override=(phase, missing)), encoding="utf-8")
            run("cabal-refuse-" + name, [cabal, "v2-build", "--offline", "exe:ci-toolchain-probe"],
                cwd=package, child_env=dict(env, CABAL_CONFIG=str(refused)), expected_refusal=missing)
        cabal_env = dict(env, CABAL_CONFIG=str(config))
        run("cabal-build", [cabal, "v2-build", "--offline", "exe:ci-toolchain-probe"], cwd=package, child_env=cabal_env)
        if run("cabal-run", [cabal, "v2-run", "--offline", "-v0", "exe:ci-toolchain-probe"], cwd=package, child_env=cabal_env).strip() != "42":
            raise RuntimeError("Cabal foreign-C runtime output mismatch")
        receipt["hsc_source_invocations"] = verify_hsc_route(output, hsc2hs, forced_hsc)
        receipt["hsc_trace_sha256"] = hashlib.sha256((output / "hsc2hs-invocations.jsonl").read_bytes()).hexdigest()
        if original.read_bytes() != original_bytes:
            raise RuntimeError("original Cabal configuration changed during bootstrap")
        if any(c in str(config) for c in "\n\r"):
            raise ValueError("invalid environment-file path")
        with github_env.open("a", encoding="utf-8", newline="\n") as stream:
            stream.write("CABAL_CONFIG=" + str(config) + "\n")
        receipt["success"] = True
        save()
        return receipt
    except Exception:
        save()
        raise


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--github-env", type=Path, required=True)
    args = parser.parse_args()
    if platform.system() != "Darwin":
        parser.error("this bootstrap requires macOS")
    receipt = bootstrap(args.output_dir, args.github_env, dict(os.environ))
    print(json.dumps({"success": receipt["success"], "receipt": str(args.output_dir / "receipt.json")}))


if __name__ == "__main__":
    main()

"""Package a built VisiCalc website, including its Rust runtime, for release."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import zipfile


def package(dist: Path, output: Path, version: str, commit: str) -> Path:
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?", version):
        raise ValueError("version must be a numeric X.Y.Z with an optional prerelease suffix")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("source commit must be a full Git SHA")
    files = {}
    for path in sorted(dist.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"release input must not contain symlinks: {path}")
        if path.is_file():
            files[path.relative_to(dist).as_posix()] = path.read_bytes()
    if b"\0asm\x01\0\0\0" != files.get("visicalc_mosaic_app.wasm", b"")[:8]:
        raise ValueError("missing or invalid compiled Rust WASM runtime")
    if "index.html" not in files or not any(name.endswith(".js") for name in files):
        raise ValueError("missing production HTML/JavaScript")
    reserved = {"release.json", "README.txt"}
    if reserved.intersection(files):
        raise ValueError("build output collides with release metadata")
    files["README.txt"] = (
        "VisiCalc web preview\n\n"
        "Requires Python 3 and a modern browser. Extract this entire ZIP.\n"
        "From this directory run: python -m http.server 8080 --bind 127.0.0.1\n"
        "Open http://127.0.0.1:8080/ (do not open index.html as a file).\n"
        "No checkout, npm packages or Rust installation is required.\n"
        "Keep this directory as the server root; the engine uses a root URL.\n"
        "Save workbooks before closing. Open/Save requires browser file-picker support.\n"
        "This preview is not native-platform or accessibility acceptance.\n"
        "Updates: extract a new version separately. Keep the old folder to roll back.\n"
    ).encode()
    manifest = {"version": version, "commit": commit, "target": "web",
                "files": {name: hashlib.sha256(data).hexdigest() for name, data in files.items()}}
    files["release.json"] = (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode()
    output.mkdir(parents=True, exist_ok=True)
    archive = output / f"visicalc-{version}-web.zip"
    # Fixed metadata makes repeated packaging of the same build byte-identical.
    with zipfile.ZipFile(archive, "x", compression=zipfile.ZIP_DEFLATED) as bundle:
        for name, data in sorted(files.items()):
            entry = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            entry.compress_type = zipfile.ZIP_DEFLATED
            entry.external_attr = 0o100644 << 16
            bundle.writestr(entry, data)
    checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
    (output / "SHA256SUMS").write_text(f"{checksum}  {archive.name}\n", encoding="utf-8")
    return archive


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    args = parser.parse_args()
    print(package(args.dist, args.output, args.version, args.commit))

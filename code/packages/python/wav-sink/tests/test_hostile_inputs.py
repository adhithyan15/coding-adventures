"""Hostile input testing for CWE-20, CWE-22, CWE-190, CWE-400, and CWE-1333 (VER15)."""

from __future__ import annotations

import ast
import os
import subprocess
import sys
from pathlib import Path
from unittest.mock import patch

import pytest
from pcm_audio import MAX_WAV_BYTES, PCMBuffer, PCMFormat

from wav_sink import (
    calc_total_wav_bytes,
    check_wav_size_bounds,
    read_wav,
    to_wav_bytes,
    validate_wav_path,
    write_wav,
)


class _BytesPathLike:
    """Hostile os.PathLike returning bytes from __fspath__."""

    def __fspath__(self) -> bytes:
        return b"bad.wav"


class TestCwe20AndCwe190Validation:
    def test_bool_rejected_on_size_and_format_fields(self) -> None:
        with pytest.raises(TypeError, match="payload must be an int"):
            calc_total_wav_bytes(True)  # type: ignore[arg-type]
        assert not check_wav_size_bounds(True)  # type: ignore[arg-type]
        assert not check_wav_size_bounds(False)  # type: ignore[arg-type]

        fmt_ch = PCMFormat(sample_rate_hz=8.0)
        object.__setattr__(fmt_ch, "channel_count", True)
        with pytest.raises(TypeError, match="channel_count"):
            to_wav_bytes(PCMBuffer((0,), fmt_ch))

        fmt_w = PCMFormat(sample_rate_hz=8.0)
        object.__setattr__(fmt_w, "bit_depth", True)
        with pytest.raises(TypeError, match="bit_depth"):
            to_wav_bytes(PCMBuffer((0,), fmt_w))

        fmt_sw = PCMFormat(sample_rate_hz=8.0)
        with (
            patch.object(
                type(fmt_sw), "sample_width_bytes", property(lambda _self: True)
            ),
            pytest.raises(TypeError, match="sample_width_bytes"),
        ):
            to_wav_bytes(PCMBuffer((0,), fmt_sw))

        fmt_ir = PCMFormat(sample_rate_hz=8.0)
        object.__setattr__(fmt_ir, "integer_sample_rate", lambda: True)
        with pytest.raises(TypeError, match="integer_sample_rate"):
            to_wav_bytes(PCMBuffer((0,), fmt_ir))

    def test_huge_int_and_overflow_in_tampered_fields_fail_safely(self) -> None:
        with pytest.raises(ValueError, match="<out-of-range int>") as exc1:
            calc_total_wav_bytes(-(10**5000))
        assert len(str(exc1.value)) < 160

        buf_neg = PCMBuffer((0,), PCMFormat(sample_rate_hz=8.0))
        object.__setattr__(buf_neg, "sample_count", lambda: -(10**5000))
        with pytest.raises(ValueError, match="<out-of-range int>") as exc2:
            to_wav_bytes(buf_neg)
        assert len(str(exc2.value)) < 160

        buf_huge = PCMBuffer((0,), PCMFormat(sample_rate_hz=8.0))
        object.__setattr__(buf_huge, "sample_count", lambda: 10**5000)
        with pytest.raises(ValueError, match="<out-of-range int>") as exc3:
            to_wav_bytes(buf_huge)
        assert len(str(exc3.value)) < 160

        fmt_huge_sr = PCMFormat(sample_rate_hz=8.0)
        object.__setattr__(fmt_huge_sr, "sample_rate_hz", 10**400)
        with pytest.raises(ValueError, match="overflows float range") as exc4:
            to_wav_bytes(PCMBuffer((0,), fmt_huge_sr))
        assert len(str(exc4.value)) < 160

        fmt_sub = PCMFormat(sample_rate_hz=8.0)
        object.__setattr__(fmt_sub, "sample_rate_hz", 5e-324)
        with pytest.raises(ValueError, match="subnormal"):
            to_wav_bytes(PCMBuffer((0,), fmt_sub))

    def test_riff_32bit_overflow_rejected_by_bounds(self) -> None:
        assert not check_wav_size_bounds(calc_total_wav_bytes(2**32))
        assert not check_wav_size_bounds(MAX_WAV_BYTES + 1)


class TestCwe22PathAndSymlinkSafety:
    @pytest.mark.parametrize(
        "bad_path",
        [
            "../evil.wav",
            "sub/../evil.wav",
            "a/b/../../evil.wav",
            "evil\x00.wav",
            "",
            "   ",
            "evil.txt",
            "evil.wav.exe",
        ],
    )
    def test_rejects_traversal_null_empty_and_bad_extensions(
        self, bad_path: str
    ) -> None:
        with pytest.raises(ValueError):
            validate_wav_path(bad_path)

    def test_rejects_bytes_pathlike_and_oversized_paths(self, tmp_path: Path) -> None:
        with pytest.raises(TypeError, match="resolve to str"):
            validate_wav_path(_BytesPathLike())  # type: ignore[arg-type]
        with pytest.raises(TypeError, match="resolve to str"):
            validate_wav_path("ok.wav", base_dir=_BytesPathLike())  # type: ignore[arg-type]

        long_name = "a" * 5000 + ".wav"
        with pytest.raises(ValueError, match="MAX_PATH_LENGTH"):
            validate_wav_path(long_name)
        with pytest.raises(ValueError, match="MAX_PATH_LENGTH"):
            validate_wav_path("ok.wav", base_dir="b" * 5000)

        with pytest.raises(ValueError, match="escapes base directory"):
            validate_wav_path(tmp_path / "escape.wav", base_dir=tmp_path / "sub") if (
                tmp_path / "sub"
            ).mkdir() is None else None

    def test_rejects_symlink_and_directory_on_write_and_read(
        self, tmp_path: Path
    ) -> None:
        d = tmp_path / "dir.wav"
        d.mkdir()
        pcm = PCMBuffer((0,), PCMFormat(sample_rate_hz=8.0))
        with pytest.raises(ValueError, match="not a regular file"):
            write_wav(d, pcm)
        with pytest.raises(ValueError, match="not a regular file"):
            read_wav(d)

        f = tmp_path / "sym.wav"
        f.touch()
        with (
            patch.object(Path, "is_symlink", return_value=True),
            pytest.raises(ValueError, match="is a symlink"),
        ):
            write_wav(f, pcm)

    def test_rejects_ntfs_junction_and_relative_ancestor_symlink(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        pcm = PCMBuffer((0,), PCMFormat(sample_rate_hz=8.0))
        sub = tmp_path / "sub"
        sub.mkdir()
        target_in_sub = sub / "out.wav"

        # Mocked junction check (cross-platform coverage)
        def mock_isjunction(p: object) -> bool:
            return Path(str(p)) == sub

        with (
            patch.object(os.path, "isjunction", mock_isjunction, create=True),
            pytest.raises(ValueError, match="symlink or junction"),
        ):
            validate_wav_path(target_in_sub)

        # Relative path must anchor via abspath and catch CWD/ancestor symlink/junction
        monkeypatch.chdir(sub)
        with (
            patch.object(os.path, "isjunction", mock_isjunction, create=True),
            pytest.raises(ValueError, match="ancestor .* symlink or junction"),
        ):
            validate_wav_path("relative.wav")

        # Real NTFS directory junction on Windows
        if sys.platform == "win32":
            outside = tmp_path / "outside"
            outside.mkdir()
            junc = tmp_path / "junc"
            res = subprocess.run(
                ["cmd", "/c", "mklink", "/J", str(junc), str(outside)],
                capture_output=True,
                text=True,
                check=False,
            )
            if res.returncode == 0:
                with pytest.raises(ValueError, match="symlink or junction"):
                    write_wav(junc / "pwn.wav", pcm)
                assert not (outside / "pwn.wav").exists()

    @pytest.mark.parametrize(
        "ads_path",
        [
            "readme.txt:hidden.wav",
            "sub/carrier.txt:stream.wav",
            "dir:ads/audio.wav",
        ],
    )
    def test_rejects_ntfs_alternate_data_streams(self, ads_path: str) -> None:
        with pytest.raises(ValueError, match="alternate data stream"):
            validate_wav_path(ads_path)

    @pytest.mark.parametrize(
        "dev_path",
        [
            "NUL.wav",
            "CON.wav",
            "PRN.wave",
            "AUX.wav",
            "COM1.wav",
            "LPT9.wav",
            "sub/nul.wav",
        ],
    )
    def test_rejects_windows_reserved_device_names(self, dev_path: str) -> None:
        with pytest.raises(ValueError, match="reserved device"):
            validate_wav_path(dev_path)

    def test_tampered_sample_count_cannot_bypass_duration_bound(self) -> None:
        long_buf = PCMBuffer((0,) * 100, PCMFormat(sample_rate_hz=1.0))
        object.__setattr__(long_buf, "sample_count", lambda: 1)
        with pytest.raises(ValueError, match="MAX_DURATION_SECONDS"):
            to_wav_bytes(long_buf)


class TestCwe1333ZeroRegexImports:
    def test_no_regex_imports_in_source(self) -> None:
        pkg_src = Path(__file__).resolve().parents[1] / "src" / "wav_sink"
        for py_file in pkg_src.rglob("*.py"):
            tree = ast.parse(py_file.read_text(encoding="utf-8"))
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    assert all(alias.name != "re" for alias in node.names)
                elif isinstance(node, ast.ImportFrom):
                    assert node.module != "re"

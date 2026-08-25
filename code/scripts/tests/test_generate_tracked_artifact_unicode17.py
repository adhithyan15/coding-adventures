from __future__ import annotations

import hashlib
import sys
import unittest
import urllib.request
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import generate_tracked_artifact_unicode17 as generator


class _Response:
    def __init__(self, url: str, payload: bytes) -> None:
        self.url = url
        self.payload = payload
        self.read_limit: int | None = None

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback) -> None:
        return None

    def geturl(self) -> str:
        return self.url

    def read(self, limit: int) -> bytes:
        self.read_limit = limit
        return self.payload[:limit]


class UnicodeDownloadBoundaryTests(unittest.TestCase):
    def test_download_requires_exact_origin_url_size_and_digest(self) -> None:
        url = "https://www.unicode.org/test.txt"
        payload = b"abc"
        response = _Response(url, payload)
        with mock.patch.object(generator._HTTPS_OPENER, "open", return_value=response):
            actual = generator._download_exact(
                url,
                expected_size=len(payload),
                expected_hash=hashlib.sha256(payload).hexdigest(),
                label="test",
            )

        self.assertEqual(actual, payload)
        self.assertEqual(response.read_limit, len(payload) + 1)

    def test_download_rejects_final_url_drift(self) -> None:
        url = "https://www.unicode.org/test.txt"
        response = _Response("https://internal.example/test.txt", b"abc")
        with (
            mock.patch.object(generator._HTTPS_OPENER, "open", return_value=response),
            self.assertRaisesRegex(RuntimeError, "final URL drift"),
        ):
            generator._download_exact(
                url,
                expected_size=3,
                expected_hash=hashlib.sha256(b"abc").hexdigest(),
                label="test",
            )

    def test_download_rejects_lookalike_origin_before_open(self) -> None:
        with (
            mock.patch.object(generator._HTTPS_OPENER, "open") as open_mock,
            self.assertRaisesRegex(RuntimeError, "left the pinned HTTPS origin"),
        ):
            generator._download_exact(
                "https://www.unicode.org.evil.example/test.txt",
                expected_size=3,
                expected_hash=hashlib.sha256(b"abc").hexdigest(),
                label="test",
            )
        open_mock.assert_not_called()

    def test_redirect_handler_fails_closed(self) -> None:
        request = urllib.request.Request("https://www.unicode.org/test.txt")
        with self.assertRaisesRegex(RuntimeError, "refused redirect"):
            generator._RejectRedirects().redirect_request(
                request,
                None,
                302,
                "Found",
                {},
                "https://internal.example/test.txt",
            )


if __name__ == "__main__":
    unittest.main()

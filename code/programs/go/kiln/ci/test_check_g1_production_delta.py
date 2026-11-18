"""Bounded unit checks for the experiment's compiled-source identity gate."""

import contextlib
import io
import unittest
from pathlib import Path
from unittest import mock

import check_g1_production_delta as check


class ValidateDeltaTests(unittest.TestCase):
    def test_valid_reviewed_single_file_change(self):
        # These are the exact matcher.go blobs, also checked against the
        # pinned B/C repositories by the workflow's integration invocation.
        check.validate_delta(
            {check.TARGET: check.BASELINE_BLOB},
            {check.TARGET: check.CANDIDATE_BLOB},
        )

    def test_missing_delta_is_not_misreported_as_unexpected(self):
        with self.assertRaisesRegex(check.CheckError, "candidate source delta missing"):
            check.validate_delta({check.TARGET: check.BASELINE_BLOB}, {check.TARGET: check.BASELINE_BLOB})

    def test_unrelated_or_extra_delta_is_rejected(self):
        with self.assertRaisesRegex(check.CheckError, "unexpected compiled production Go delta"):
            check.validate_delta(
                {check.TARGET: check.BASELINE_BLOB, "code/packages/go/other/other.go": "a"},
                {check.TARGET: check.CANDIDATE_BLOB, "code/packages/go/other/other.go": "b"},
            )

    def test_missing_target_is_reported_as_missing_delta(self):
        with self.assertRaisesRegex(check.CheckError, "candidate source delta missing"):
            check.validate_delta({check.TARGET: check.BASELINE_BLOB}, {})


class GitFailureTests(unittest.TestCase):
    def test_missing_git_is_a_required_tool_error(self):
        with mock.patch.object(check.shutil, "which", return_value=None):
            with mock.patch.object(check.sys, "argv", ["check", "a", "b"]):
                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    self.assertEqual(check.main(), 2)
                self.assertIn("required tool error", stderr.getvalue())

    def test_git_execution_error_is_not_a_source_delta(self):
        with mock.patch.object(check.subprocess, "run", side_effect=FileNotFoundError("git")):
            with self.assertRaisesRegex(check.CheckError, "required tool error"):
                check.production_blobs(Path("/not-used"))

    def test_git_nonzero_is_reported_as_enumeration_error(self):
        failed = mock.Mock(returncode=128, stdout=b"", stderr=b"not a repository")
        with mock.patch.object(check.subprocess, "run", return_value=failed):
            with self.assertRaisesRegex(check.CheckError, "source enumeration error"):
                check.production_blobs(Path("/not-a-repository"))


if __name__ == "__main__":
    unittest.main()

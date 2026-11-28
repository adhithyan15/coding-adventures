"""Exercise the exact Hackage bootstrap block used by the CI workflow."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
WORKFLOW_PATH = REPOSITORY_ROOT / ".github" / "workflows" / "ci.yml"
BEGIN_MARKER = "# HASKELL_CABAL_HTTPS_BOOTSTRAP_BEGIN"
END_MARKER = "# HASKELL_CABAL_HTTPS_BOOTSTRAP_END"
EXPECTED_CONFIG = (
    b"repository hackage.haskell.org\n"
    b"  url: https://hackage.haskell.org/\n"
    b"  secure: True\n"
    b"store-dir: C:\\cabal store\\store\n"
    b"install-method: copy\n"
    b"overwrite-policy: always\n"
)


def extract_bootstrap_block() -> str:
    """Return the inline Bash source between its unique workflow markers."""
    lines = WORKFLOW_PATH.read_text(encoding="utf-8").splitlines()
    begin = [index for index, line in enumerate(lines) if line.strip() == BEGIN_MARKER]
    end = [index for index, line in enumerate(lines) if line.strip() == END_MARKER]
    if len(begin) != 1 or len(end) != 1 or begin[0] >= end[0]:
        raise AssertionError("workflow must contain one ordered Cabal bootstrap marker pair")

    block = textwrap.dedent("\n".join(lines[begin[0] + 1 : end[0]])).strip("\n")
    if not block:
        raise AssertionError("the marked Cabal bootstrap block is empty")
    return block


def run_bootstrap(
    block: str,
    *,
    runner_temp: str | None,
    github_env: str | None,
    cabal_store_dir: str | None = r"C:\cabal store\store",
    runner_os: str | None = "Windows",
    path_prefix: Path | None = None,
) -> subprocess.CompletedProcess[bytes]:
    """Run the extracted production block with GitHub's fail-fast Bash flags."""
    bash = shutil.which("bash")
    if bash is None:
        raise unittest.SkipTest("bash is required to exercise the workflow's shell block")
    mktemp = shutil.which("mktemp")
    if mktemp is None:
        raise unittest.SkipTest("mktemp is required to exercise the workflow's shell block")

    env = {
        key: os.environ[key]
        for key in ("SystemRoot", "WINDIR", "COMSPEC", "TEMP", "TMP")
        if key in os.environ
    }
    path_entries = [str(Path(bash).parent), str(Path(mktemp).parent)]
    if path_prefix is not None:
        path_entries.insert(0, str(path_prefix))
    system32 = Path(env.get("SystemRoot", "")) / "System32"
    if system32.is_dir():
        path_entries.append(str(system32))
    env["PATH"] = os.pathsep.join(dict.fromkeys(path_entries))
    if runner_temp is not None:
        env["RUNNER_TEMP"] = runner_temp
    if github_env is not None:
        env["GITHUB_ENV"] = github_env
    if cabal_store_dir is not None:
        env["CABAL_STORE_DIR"] = cabal_store_dir
    if runner_os is not None:
        env["RUNNER_OS"] = runner_os

    return subprocess.run(
        [bash, "-e", "-o", "pipefail", "-c", block],
        cwd=REPOSITORY_ROOT,
        env=env,
        capture_output=True,
        check=False,
        timeout=15,
    )


class HaskellCabalConfigBootstrapTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.block = extract_bootstrap_block()

    def assert_failed_without_assignment(
        self,
        result: subprocess.CompletedProcess[bytes],
        env_path: Path | None,
    ) -> None:
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn(b"CABAL_CONFIG=", result.stdout)
        if env_path is not None and env_path.is_file():
            self.assertNotIn(b"CABAL_CONFIG=", env_path.read_bytes())

    def test_writes_exact_config_and_appends_native_path(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cabal bootstrap ") as temp:
            root = Path(temp)
            runner_temp = root / "runner temp with spaces"
            runner_temp.mkdir()
            env_path = root / "github env.txt"
            old_content = b"EXISTING=value\nSECOND=kept\n"
            env_path.write_bytes(old_content)

            result = run_bootstrap(
                self.block,
                runner_temp=str(runner_temp),
                github_env=str(env_path),
            )

            self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
            env_bytes = env_path.read_bytes()
            self.assertTrue(env_bytes.startswith(old_content))
            appended = env_bytes[len(old_content) :]
            self.assertEqual(appended.count(b"CABAL_CONFIG="), 1)
            self.assertTrue(appended.endswith(b"\n"))
            value = appended.removeprefix(b"CABAL_CONFIG=").removesuffix(b"\n")
            config_path = Path(value.decode("utf-8"))
            self.assertTrue(config_path.is_absolute(), value)
            self.assertIn(" ", str(config_path))
            resolved_root = runner_temp.resolve()
            resolved_config = config_path.resolve()
            self.assertEqual(resolved_config.parent.parent, resolved_root)
            self.assertEqual(config_path.read_bytes(), EXPECTED_CONFIG)
            self.assertFalse(config_path.read_bytes().startswith(b"\xef\xbb\xbf"))

            # A second invocation must create a distinct private config and append,
            # rather than replace, the first assignment and pre-existing content.
            second = run_bootstrap(
                self.block,
                runner_temp=str(runner_temp),
                github_env=str(env_path),
            )
            self.assertEqual(second.returncode, 0, second.stderr.decode(errors="replace"))
            second_bytes = env_path.read_bytes()
            self.assertTrue(second_bytes.startswith(env_bytes))
            assignments = [
                line.removeprefix(b"CABAL_CONFIG=")
                for line in second_bytes.splitlines()
                if line.startswith(b"CABAL_CONFIG=")
            ]
            self.assertEqual(len(assignments), 2)
            self.assertNotEqual(assignments[0], assignments[1])
            self.assertTrue(all(Path(item.decode("utf-8")).is_file() for item in assignments))

            unix_env = root / "github env unix"
            unix_env.write_bytes(b"KEEP=1\n")
            unix_result = run_bootstrap(
                self.block,
                runner_temp=str(runner_temp),
                github_env=str(unix_env),
                runner_os="Linux",
            )
            self.assertEqual(unix_result.returncode, 0, unix_result.stderr.decode(errors="replace"))
            unix_assignment = next(
                line.removeprefix(b"CABAL_CONFIG=")
                for line in unix_env.read_bytes().splitlines()
                if line.startswith(b"CABAL_CONFIG=")
            )
            unix_config = Path(unix_assignment.decode("utf-8")).read_bytes()
            self.assertEqual(
                unix_config,
                b"repository hackage.haskell.org\n"
                b"  url: https://hackage.haskell.org/\n"
                b"  secure: True\n"
                b"store-dir: C:\\cabal store\\store\n",
            )

    def test_missing_inputs_fail_before_creating_config(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            runner_temp = root / "runner"
            runner_temp.mkdir()
            env_path = root / "env"
            env_path.write_bytes(b"KEEP=1\n")

            no_runner = run_bootstrap(
                self.block,
                runner_temp=None,
                github_env=str(env_path),
            )
            self.assert_failed_without_assignment(no_runner, env_path)
            self.assertEqual(list(runner_temp.iterdir()), [])

            no_env = run_bootstrap(
                self.block,
                runner_temp=str(runner_temp),
                github_env=None,
            )
            self.assert_failed_without_assignment(no_env, None)
            self.assertEqual(list(runner_temp.iterdir()), [])

            no_store = run_bootstrap(
                self.block,
                runner_temp=str(runner_temp),
                github_env=str(env_path),
                cabal_store_dir=None,
            )
            self.assert_failed_without_assignment(no_store, env_path)
            self.assertEqual(list(runner_temp.iterdir()), [])

            no_os = run_bootstrap(
                self.block,
                runner_temp=str(runner_temp),
                github_env=str(env_path),
                runner_os=None,
            )
            self.assert_failed_without_assignment(no_os, env_path)
            self.assertEqual(list(runner_temp.iterdir()), [])

            unsupported_os = run_bootstrap(
                self.block,
                runner_temp=str(runner_temp),
                github_env=str(env_path),
                runner_os="Plan9",
            )
            self.assert_failed_without_assignment(unsupported_os, env_path)
            self.assertEqual(list(runner_temp.iterdir()), [])

    def test_cr_and_lf_path_injection_is_rejected(self) -> None:
        for injected in ("\r", "\n"):
            with self.subTest(character=repr(injected)), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                runner_temp = root / "runner"
                runner_temp.mkdir()
                env_path = root / "env"
                env_path.write_bytes(b"KEEP=1\n")

                bad_runner = run_bootstrap(
                    self.block,
                    runner_temp=f"{runner_temp}{injected}INJECTED=1",
                    github_env=str(env_path),
                )
                self.assert_failed_without_assignment(bad_runner, env_path)
                self.assertEqual(list(runner_temp.iterdir()), [])

                bad_env = run_bootstrap(
                    self.block,
                    runner_temp=str(runner_temp),
                    github_env=f"{env_path}{injected}INJECTED=1",
                )
                self.assert_failed_without_assignment(bad_env, env_path)
                self.assertEqual(list(runner_temp.iterdir()), [])

                bad_store = run_bootstrap(
                    self.block,
                    runner_temp=str(runner_temp),
                    github_env=str(env_path),
                    cabal_store_dir=f"C:\\store{injected}INJECTED=1",
                )
                self.assert_failed_without_assignment(bad_store, env_path)
                self.assertEqual(list(runner_temp.iterdir()), [])

    def test_directory_creation_and_environment_append_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            not_a_directory = root / "runner-file"
            not_a_directory.write_bytes(b"x")
            env_path = root / "env"
            env_path.write_bytes(b"KEEP=1\n")

            mkdir_failure = run_bootstrap(
                self.block,
                runner_temp=str(not_a_directory),
                github_env=str(env_path),
            )
            self.assert_failed_without_assignment(mkdir_failure, env_path)
            self.assertEqual(env_path.read_bytes(), b"KEEP=1\n")

            runner_temp = root / "runner"
            runner_temp.mkdir()
            env_is_directory = root / "env-directory"
            env_is_directory.mkdir()
            append_failure = run_bootstrap(
                self.block,
                runner_temp=str(runner_temp),
                github_env=str(env_is_directory),
            )
            self.assertNotEqual(append_failure.returncode, 0)
            self.assertNotIn(b"CABAL_CONFIG=", append_failure.stdout)
            self.assertEqual(list(env_is_directory.iterdir()), [])
            configs = list(runner_temp.glob("cabal-https.*/config"))
            self.assertEqual(len(configs), 1)
            self.assertEqual(configs[0].read_bytes(), EXPECTED_CONFIG)

    def test_config_write_failure_does_not_append_success(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cabal fake mktemp ") as temp:
            root = Path(temp)
            runner_temp = root / "runner temp"
            runner_temp.mkdir()
            env_path = root / "github env"
            prior_env = b"KEEP=1\n"
            env_path.write_bytes(prior_env)
            fake_bin = root / "fake bin"
            fake_bin.mkdir()
            called = runner_temp / "fake-mktemp-called"
            fake_mktemp = fake_bin / "mktemp"
            fake_mktemp.write_text(
                "#!/bin/sh\n"
                'printf "%s\\n" "$RUNNER_TEMP/cabal-https.fixture" > '
                '"$RUNNER_TEMP/fake-mktemp-called"\n'
                'mkdir "$RUNNER_TEMP/cabal-https.fixture"\n'
                'mkdir "$RUNNER_TEMP/cabal-https.fixture/config"\n'
                'printf "%s\\n" "$RUNNER_TEMP/cabal-https.fixture"\n',
                encoding="utf-8",
                newline="\n",
            )
            fake_mktemp.chmod(0o755)

            result = run_bootstrap(
                self.block,
                runner_temp=str(runner_temp),
                github_env=str(env_path),
                path_prefix=fake_bin,
            )

            self.assertTrue(called.is_file(), "the controlled mktemp witness was not used")
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn(b"CABAL_CONFIG=", result.stdout)
            self.assertEqual(env_path.read_bytes(), prior_env)
            self.assertTrue((runner_temp / "cabal-https.fixture" / "config").is_dir())

    def test_workflow_disables_setup_update_and_updates_before_index_query(self) -> None:
        workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
        self.assertIn(
            "python3 -m unittest discover -s code/scripts/tests -p "
            "'test_haskell_cabal_config_bootstrap.py'",
            workflow,
        )
        setup_name = workflow.index("- name: Set up Haskell\n")
        setup_end = workflow.find("\n      - name:", setup_name + 1)
        self.assertNotEqual(setup_end, -1)
        setup_step = workflow[setup_name:setup_end]
        self.assertIn("cabal-update: false", setup_step)
        self.assertIn("id: haskell", setup_step)

        bootstrap_name = workflow.index("- name: Set up Haskell package index transport\n")
        verify_name = workflow.index("- name: Verify Haskell package index\n")
        self.assertLess(setup_name, bootstrap_name)
        self.assertLess(setup_name, verify_name)
        self.assertLess(bootstrap_name, verify_name)
        bootstrap_end = workflow.find("\n      - name:", bootstrap_name + 1)
        self.assertNotEqual(bootstrap_end, -1)
        bootstrap_step = workflow[bootstrap_name:bootstrap_end]
        self.assertIn("CABAL_STORE_DIR: ${{ steps.haskell.outputs.cabal-store }}", bootstrap_step)
        self.assertIn("install-method: copy", bootstrap_step)
        self.assertIn("overwrite-policy: always", bootstrap_step)
        verify_end = workflow.find("\n      - name:", verify_name + 1)
        self.assertNotEqual(verify_end, -1)
        verify_step = workflow[verify_name:verify_end]
        update = verify_step.index("cabal update")
        query = verify_step.index("cabal list --simple-output hspec")
        self.assertLess(update, query)


if __name__ == "__main__":
    unittest.main()

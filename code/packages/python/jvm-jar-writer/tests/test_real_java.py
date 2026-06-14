"""End-to-end test: build a real Java ``Hello`` JAR and run it.

This test compiles a tiny ``Hello.java`` with real ``javac`` to
get a valid ``.class`` file, then bundles it via our writer into
a JAR with ``Main-Class: Hello``, then invokes ``java -jar``.

If ``javac`` is missing the test skips.  If ``javac`` is present
but ``java`` isn't, the test also skips.  When both are present,
this is the single strongest correctness proof we have for the
writer: real Oracle/Adoptium JDK accepts our JAR.
"""

from __future__ import annotations

import shutil
import subprocess
import tempfile
import textwrap
from pathlib import Path

import pytest

from jvm_jar_writer import JarManifest, write_jar


def _has_jdk() -> bool:
    return shutil.which("javac") is not None and shutil.which("java") is not None


requires_jdk = pytest.mark.skipif(
    not _has_jdk(),
    reason="javac/java not on PATH",
)


@requires_jdk
def test_hello_jar_runs_on_real_java() -> None:
    """Compile a Hello class with javac, bundle it via write_jar,
    invoke java -jar, assert on stdout."""
    with tempfile.TemporaryDirectory() as tmp:
        td = Path(tmp)
        src = td / "Hello.java"
        src.write_text(
            textwrap.dedent("""
                public class Hello {
                    public static void main(String[] args) {
                        System.out.println("hello-from-jar");
                    }
                }
            """).strip()
        )

        # 1. Compile with real javac.
        compile_result = subprocess.run(
            ["javac", str(src)],
            cwd=td,
            capture_output=True,
            timeout=30,
            check=False,
        )
        assert compile_result.returncode == 0, compile_result.stderr

        class_bytes = (td / "Hello.class").read_bytes()
        # Sanity: the JVM magic.
        assert class_bytes[:4] == b"\xca\xfe\xba\xbe"

        # 2. Bundle via our writer.
        jar_bytes = write_jar(
            classes=(("Hello.class", class_bytes),),
            manifest=JarManifest(main_class="Hello"),
        )
        jar_path = td / "Hello.jar"
        jar_path.write_bytes(jar_bytes)

        # 3. Run with real java -jar.
        run_result = subprocess.run(
            ["java", "-jar", str(jar_path)],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        assert run_result.returncode == 0, (
            f"java -jar rejected the JAR.\n"
            f"  stdout: {run_result.stdout!r}\n"
            f"  stderr: {run_result.stderr!r}"
        )
        assert "hello-from-jar" in run_result.stdout

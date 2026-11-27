import json
import unittest
from pathlib import Path

from unix_history import DescriptorTable, FileSystem, Kernel, run_command

FIXTURES = (
    Path(__file__).resolve().parents[4]
    / "specs/fixtures/historical-unix-v1/file-read.json"
)


class UnixHistoryTests(unittest.TestCase):
    def test_shared_traces(self) -> None:
        for fixture in json.loads(FIXTURES.read_text(encoding="utf-8")):
            with self.subTest(command=fixture["command"]):
                self.assertEqual(
                    run_command(fixture["command"]),
                    {"output": fixture["output"], "events": fixture["events"]},
                )

    def test_lookup(self) -> None:
        fs = FileSystem()
        self.assertEqual(fs.resolve("//etc/../etc/./notes").number, 5)
        self.assertEqual(fs.resolve("/hello/.."), "ENOTDIR")
        self.assertEqual(fs.resolve("a\0b"), "EINVAL")

    def test_descriptor_reuse(self) -> None:
        descriptors = DescriptorTable(2)
        self.assertEqual(descriptors.open(2), 0)
        self.assertEqual(descriptors.open(2), 1)
        self.assertIsNone(descriptors.open(2))
        descriptors.advance(0, 4)
        self.assertEqual(descriptors.get(1).offset, 0)
        self.assertFalse(descriptors.close(-1))
        self.assertTrue(descriptors.close(0))
        self.assertEqual(descriptors.open(3), 0)
        self.assertEqual(descriptors.get(0).offset, 0)

    def test_kernel_errors(self) -> None:
        kernel = Kernel()
        self.assertEqual(kernel.open("/")["error"], "EISDIR")
        self.assertEqual(kernel.read(0, 8)[0]["error"], "EBADF")
        kernel.open("/hello")
        self.assertEqual(kernel.read(0, -1)[0]["error"], "EINVAL")
        self.assertEqual(kernel.read(0, 2)[1], b"He")
        self.assertEqual(run_command("cat")["events"], [])

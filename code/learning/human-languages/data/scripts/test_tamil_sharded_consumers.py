import inspect
import json
import os
import runpy
import unittest


HERE = os.path.dirname(os.path.abspath(__file__))


def script_namespace(name):
    return runpy.run_path(os.path.join(HERE, name), run_name="inventory_loader_test")


class TamilShardedConsumerTest(unittest.TestCase):
    def test_drizzle_reads_the_shard_native_inventory(self):
        # What this pins: the drizzle author loads Tamil from the shard
        # directory (tamil.d/letters/*.json, tamil.d/marks/*.json), one entry
        # per shard file, in shard order. The expected entries are read from
        # those files themselves, so adding a letter shard (the inventory grew
        # from 25 to 30 letters) moves both sides together instead of leaving
        # a stale literal count behind; a loader that dropped, duplicated or
        # reordered a shard would still fail.
        namespace = script_namespace("author_drizzle_segments.py")
        for section, key in (("letters", "glyph"), ("marks", "mark")):
            shard_dir = os.path.join(HERE, "tamil.d", section)
            expected = []
            for name in sorted(os.listdir(shard_dir)):
                if name.endswith(".json"):
                    with open(os.path.join(shard_dir, name), encoding="utf-8") as handle:
                        expected.append(json.load(handle)[key])
            self.assertTrue(expected, section)
            loaded = [entry[key] for entry in namespace["SCRIPT"][section]]
            self.assertEqual(loaded, expected, section)

    def test_recognition_builder_reads_the_shard_native_inventory(self):
        namespace = script_namespace("author_recognition_segments.py")
        source = inspect.getsource(namespace["build"])
        self.assertIn('S = load_script(HL, cfg["script"])', source)

    def test_letter_ledger_keeps_logical_tamil_json_provenance(self):
        namespace = script_namespace("propose_letter_ledger.py")
        families = namespace["derived_families"]("TAMIL")
        self.assertTrue(families)
        self.assertTrue(all(family["source"].startswith("tamil.json:") for family in families))


if __name__ == "__main__":
    unittest.main()

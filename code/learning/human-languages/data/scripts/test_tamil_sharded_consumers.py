import inspect
import os
import runpy
import unittest


HERE = os.path.dirname(os.path.abspath(__file__))


def script_namespace(name):
    return runpy.run_path(os.path.join(HERE, name), run_name="inventory_loader_test")


class TamilShardedConsumerTest(unittest.TestCase):
    def test_drizzle_reads_the_shard_native_inventory(self):
        namespace = script_namespace("author_drizzle_segments.py")
        self.assertEqual(len(namespace["SCRIPT"]["letters"]), 25)
        self.assertEqual(len(namespace["SCRIPT"]["marks"]), 9)

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

from __future__ import annotations

import importlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

books = importlib.import_module("build_human_language_book_catalog")


class HumanLanguageBookCatalogTests(unittest.TestCase):
    def test_writes_catalog_for_pdfs_that_exist(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pdf_dir = root / "pdfs"
            output_dir = root / "site"
            pdf_dir.mkdir()
            (pdf_dir / "spanish.pdf").write_bytes(b"%PDF-test")
            registry = root / "languages.json"
            registry.write_text(
                json.dumps(
                    {
                        "languages": [
                            {
                                "id": "spanish",
                                "name": "Spanish",
                                "family": "Romance",
                                "script": "latin",
                            },
                            {
                                "id": "persian",
                                "name": "Persian",
                                "family": "Iranian",
                                "script": "perso-arabic",
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )

            entries = books.write_catalog(pdf_dir, registry, output_dir)

            self.assertEqual([entry["id"] for entry in entries], ["spanish"])
            catalog = json.loads(
                (output_dir / "catalog.json").read_text(encoding="utf-8")
            )
            self.assertEqual(catalog["books"][0]["family"], "Romance")
            page = (output_dir / "index.html").read_text(encoding="utf-8")
            self.assertIn("1 free, etymology-first language book", page)
            self.assertIn('href="spanish.pdf" download', page)
            self.assertNotIn("Persian", page)

    def test_rejects_an_empty_pdf_directory(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pdf_dir = root / "pdfs"
            pdf_dir.mkdir()
            registry = root / "languages.json"
            registry.write_text('{"languages": []}', encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "no PDF books found"):
                books.write_catalog(pdf_dir, registry, root / "site")


if __name__ == "__main__":
    unittest.main()

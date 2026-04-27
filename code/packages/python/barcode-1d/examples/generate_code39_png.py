from __future__ import annotations

from pathlib import Path

from barcode_1d import render_png


def main() -> None:
    output = Path("/tmp/python-metal-code39.png")
    output.write_bytes(render_png("HELLO-123", symbology="code39"))
    print(output)


if __name__ == "__main__":
    main()

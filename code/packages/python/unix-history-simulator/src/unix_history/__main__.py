"""Run one simulator command and print its trace."""

import json
import sys

from . import run_command

if __name__ == "__main__":
    command = " ".join(sys.argv[1:]) if len(sys.argv) > 1 else "cat /hello"
    print(json.dumps(run_command(command), indent=2))

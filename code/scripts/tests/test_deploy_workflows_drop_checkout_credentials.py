"""Deploy workflows must not leave a write token in the checkout.

`actions/checkout` stores the job's GITHUB_TOKEN in `.git/config` unless told
`persist-credentials: false`. The Pages deploy workflows run with
`contents: write` -- the token can push to any branch -- and between the
checkout and the publish step they run third-party build code: `npm install`
or `npm ci` of hundreds of packages, Vite, Rust build scripts. Any of it can
read `.git/config`.

None of them needs the stored token. Every one publishes through
`peaceiris/actions-gh-pages` with the token passed explicitly as
`github_token`, which clones `gh-pages` into its own directory, and no step
fetches or pushes with the checkout's git. So the checkout drops it:

    checkout (persist-credentials: false)  ->  build (no token on disk)
                                           ->  peaceiris (github_token: input)

This test fails when a workflow that publishes through peaceiris has a
checkout that keeps its credentials, so a new deploy workflow copied from an
old template cannot quietly reintroduce the exposure.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path


WORKFLOWS = Path(__file__).resolve().parents[3] / ".github" / "workflows"
PUBLISHER = "peaceiris/actions-gh-pages@"


CHECKOUT = re.compile(r"^(?P<indent>\s*)(?P<dash>- )?uses: actions/checkout@")


def checkout_steps(text: str):
    """Yield (line number, step text) for each `actions/checkout` step.

    Read as text rather than parsed as YAML, so the test needs nothing beyond
    the standard library. A step runs from its `uses:` line until a line
    indented less than the step's keys, or the next `- ` item at the step's
    own level.
    """
    lines = text.splitlines()
    for number, line in enumerate(lines):
        match = CHECKOUT.match(line)
        if not match:
            continue
        key_indent = len(match["indent"]) + (2 if match["dash"] else 0)
        block = [line]
        for following in lines[number + 1 :]:
            stripped = following.strip()
            indent = len(following) - len(following.lstrip())
            if stripped and (
                indent < key_indent
                or (indent == key_indent - 2 and stripped.startswith("- "))
            ):
                break
            block.append(following)
        yield number + 1, "\n".join(block)


class DeployWorkflowsDropCheckoutCredentialsTests(unittest.TestCase):
    def test_publishing_workflows_exist(self) -> None:
        # Guards the test itself: if the publisher were renamed, every check
        # below would pass vacuously.
        publishing = [
            path
            for path in WORKFLOWS.glob("*.yml")
            if PUBLISHER in path.read_text(encoding="utf-8")
        ]
        self.assertGreaterEqual(len(publishing), 10)

    def test_checkouts_in_publishing_workflows_drop_credentials(self) -> None:
        for path in sorted(WORKFLOWS.glob("*.yml")):
            text = path.read_text(encoding="utf-8")
            if PUBLISHER not in text:
                continue
            for line, step in checkout_steps(text):
                with self.subTest(workflow=path.name, line=line):
                    self.assertRegex(
                        step,
                        r"(?m)^\s+persist-credentials:\s*false\s*$",
                        f"{path.name}:{line} checks out with the write token "
                        "left in .git/config; add `persist-credentials: false`",
                    )

    def test_step_reader_sees_both_step_shapes(self) -> None:
        text = (
            "    steps:\n"
            "      - uses: actions/checkout@v7\n"
            "        with:\n"
            "          persist-credentials: false\n"
            "      - name: Checkout again\n"
            "        uses: actions/checkout@v7\n"
            "      - run: echo persist-credentials: false\n"
        )
        steps = list(checkout_steps(text))
        self.assertEqual([line for line, _ in steps], [2, 6])
        self.assertIn("persist-credentials: false", steps[0][1])
        self.assertNotIn("persist-credentials", steps[1][1])


if __name__ == "__main__":
    unittest.main()

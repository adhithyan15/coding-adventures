"""Contract tests for FM-B029's single authoritative pull-request CI path."""

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[3]
CI_WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"
BABYSIT_SKILL = ROOT / ".claude" / "skills" / "babysit-pr" / "SKILL.md"


class FormeCiAuthorityTests(unittest.TestCase):
    def test_feature_branches_do_not_start_a_duplicate_push_suite(self) -> None:
        workflow = CI_WORKFLOW.read_text(encoding="utf-8")
        trigger = workflow.split("on:\n", 1)[1].split("\npermissions:", 1)[0]
        self.assertIn("  push:\n    branches: [main]\n", trigger)
        self.assertIn("  pull_request:\n    branches: [main]\n", trigger)
        self.assertNotIn("branches: ['**']", trigger)

    def test_cancelled_workflow_cannot_emit_a_stale_final_gate(self) -> None:
        workflow = CI_WORKFLOW.read_text(encoding="utf-8")
        gate = workflow.split("  ci-gate:\n", 1)[1]
        self.assertIn("    if: always()\n", gate)
        self.assertIn("      pull-requests: read\n", gate)
        self.assertNotIn("      - name: Classify the pull-request head\n", gate)
        self.assertNotIn("steps.current-pr-head", gate)
        self.assertIn(
            "      - name: Verify required jobs succeeded\n"
            "        if: always()\n",
            gate,
        )
        self.assertIn("EVENT_NAME: ${{ github.event_name }}", gate)
        self.assertIn("REPOSITORY: ${{ github.repository }}", gate)
        self.assertIn("PR_NUMBER: ${{ github.event.pull_request.number }}", gate)
        self.assertIn("EVENT_HEAD_SHA: ${{ github.event.pull_request.head.sha }}", gate)
        self.assertIn(
            'gh api "repos/$REPOSITORY/pulls/$PR_NUMBER" --jq \'.head.sha\'', gate
        )
        self.assertIn('if [ "$current_head" != "$EVENT_HEAD_SHA" ]; then', gate)
        self.assertIn("exit 0", gate)
        self.assertIn('for r in "$DETECT_RESULT"', gate)
        self.assertIn(
            "name: ${{ github.event_name == 'pull_request' && 'CI gate' || 'CI push gate' }}",
            gate,
        )

    def test_babysitter_pins_required_checks_to_the_current_head(self) -> None:
        skill = BABYSIT_SKILL.read_text(encoding="utf-8")
        self.assertIn("headRefOid", skill)
        self.assertIn("git rev-parse HEAD", skill)
        self.assertIn("gh pr checks \"$pr\" --required --json", skill)
        self.assertIn("expected_head", skill)
        self.assertIn("mergeCommit", skill)
        self.assertNotIn("gh run cancel", skill)


if __name__ == "__main__":
    unittest.main()

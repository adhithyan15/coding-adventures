---
category: CI & GitHub Actions
---

# Rerunning old CI builds requires regenerating expired build-plan artifacts

PR #17170's Windows setup failed while fetching `qpdf`. Rerunning only the
failed jobs three days later passed that setup but failed at "Download build
plan": the original `build-plan` artifact had expired under its one-day
retention rule. GitHub reused the successful detect job instead of uploading
a replacement plan. The artifact API confirmed that no artifacts remained.

When a delayed retry needs a short-lived artifact from an earlier job, check
that artifact's availability first. If it has expired, rerun the full workflow
so detection regenerates the plan before the platform builds. Keep required
checks tied to the unchanged PR head; do not bypass the artifact download or
describe previous platform success as success for the new attempt.

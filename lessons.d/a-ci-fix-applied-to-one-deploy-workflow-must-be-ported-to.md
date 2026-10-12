---
category: CI & GitHub Actions
---

# A CI fix applied to one deploy workflow must be ported to its sibling workflows

**What happened:** on PR #17185 the landing page's `deploy-root.yml` "build" job
failed at `npm run deploy:dry-run` with `GitHub API returned 403: API rate limit
exceeded for 20.161.28.182`. The dry run reads repository metadata through the
GitHub REST API. Without a token those reads share the hosted runner IP's
anonymous quota (60 requests an hour), which other jobs on the same IP can
exhaust at any time.

`deploy-blog.yml` already passed `GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}` to
its identical dry-run step, with a comment explaining why. `deploy-root.yml` was
never given the same change.

**Fix:** port the same `env` block to `deploy-root.yml`. The job's token is
still limited to `permissions: contents: read`, and the dry run only uses the
read boundary.

**Do differently:** when you fix a step that several workflows share (the Forme
deploy dry run, toolchain installs, cache keys), grep `.github/workflows/` for
the same step and fix every copy in the same change.

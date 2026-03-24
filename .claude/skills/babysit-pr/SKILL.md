---
description: >
  Watch a PR for CI failures and merge conflicts, fix them, and push.
  Use after creating or pushing to a PR. Sets a recurring 3-minute timer
  to monitor CI status until everything is green and conflict-free.
  Trigger: "babysit this PR", "watch the PR", "monitor CI", or after
  any PR creation/push when the user wants ongoing monitoring.
user_invocable: true
---

# Babysit PR

Monitor a pull request's CI checks and merge conflict status on a recurring
3-minute interval. Investigate and fix CI failures, resolve merge conflicts,
push fixes, and keep watching until the PR is fully green.

## Instructions

### Step 1: Identify the PR

- If a PR number or URL was provided as an argument, use that.
- Otherwise, detect the current branch and find its open PR:
  ```
  gh pr view --json number,url,headRefName,state
  ```
- If no PR exists for the current branch, tell the user and stop.
- Store the PR number for all subsequent checks.

### Step 2: Run the first check immediately

Perform the **PR Health Check** (described below) right away — don't wait
for the first timer tick.

### Step 3: Set up the recurring timer

Use `CronCreate` to schedule a recurring check every 3 minutes:
- **cron**: `*/3 * * * *`
- **prompt**: The check prompt below (substituting the actual PR number)

**Check prompt to schedule:**
```
Check PR #<NUMBER> in this repo for CI and merge conflict status.

1. Run: gh pr checks <NUMBER>
   - If any check has FAILED or ERROR status, investigate the failure:
     a. Get the failed run ID: gh run view <run-id> --log-failed
     b. Read the error logs to understand what broke
     c. Fix the code causing the failure
     d. Commit the fix with message: "fix(ci): <description of what was fixed>"
     e. Push the fix: git push

2. Run: gh pr view <NUMBER> --json mergeable,mergeStateStatus
   - If mergeable is "CONFLICTING":
     a. Fetch latest main: git fetch origin main
     b. Rebase onto main: git rebase origin/main
     c. Resolve any conflicts (prefer keeping our changes when intent is clear, otherwise investigate both sides)
     d. Continue rebase: git rebase --continue
     e. Push the resolution: git push --force-with-lease

3. If ALL checks pass (green) AND no merge conflicts exist:
   - Report success to the user: "PR #<NUMBER> is green and conflict-free!"
   - Cancel the recurring timer using CronDelete with the job ID

4. If checks are still PENDING, report status and wait for next timer tick.
```

### Step 4: Report initial status

After the first check and timer setup, tell the user:
- Current CI status (passing/failing/pending)
- Whether merge conflicts exist
- That you've set up a 3-minute recurring check
- Remind them the timer auto-expires after 3 days (CronCreate limit)

## PR Health Check procedure

This is what runs on each check cycle:

1. **Check CI status**: `gh pr checks <NUMBER>`
2. **Check merge status**: `gh pr view <NUMBER> --json mergeable,mergeStateStatus`
3. **If CI failed**: investigate logs, fix code, commit, push
4. **If merge conflict**: rebase onto main, resolve conflicts, push
5. **If all green + no conflicts**: cancel timer, report success
6. **If pending**: wait for next cycle

## Important notes

- Always use `--force-with-lease` (never `--force`) when pushing after rebase
- When fixing CI, make small focused commits — one fix per failure
- If a CI fix requires changes you're unsure about, ask the user instead of guessing
- If rebase conflicts are complex or ambiguous, ask the user for guidance
- The cron job auto-expires after 3 days per CronCreate limits

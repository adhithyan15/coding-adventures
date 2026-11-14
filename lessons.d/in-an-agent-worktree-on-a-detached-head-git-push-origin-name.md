---
category: Repo policy / workflow reminders
---

# In an agent worktree on a detached HEAD, git push origin NAME pushes the stale local branch ref, not the commits; push HEAD:refs/heads/NAME and verify the PR's headRefOid

On PR #19343 the work was committed in an agent worktree whose HEAD was
detached. A branch of the intended name existed (created earlier, pointing at
an older commit), and `git push origin <branch>` pushed THAT local ref: the
push succeeded and printed no error, but the commits were not on the remote
branch, so the PR kept showing the stale head and CI ran on old code.

Do this instead. Check `git branch --show-current` (empty means detached) before
committing, and either `git switch -c <branch>` first or push the commits you
actually made with an explicit refspec, `git push origin HEAD:refs/heads/<branch>`.
Then verify rather than trust the exit status: compare `git rev-parse HEAD` with
the PR's head, `gh pr view <n> --json headRefOid -q .headRefOid`, and with CI's
checkout sha. `babysit-pr` already pins the expected head; a head that is not the
commit just pushed is the signal that this happened.

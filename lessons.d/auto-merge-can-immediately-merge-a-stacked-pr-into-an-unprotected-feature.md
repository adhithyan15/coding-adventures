# Auto-merge can immediately merge a stacked PR into an unprotected feature base

`gh pr merge --auto` does not mean “wait for the parent PR.” It asks GitHub to
merge into the PR's current base whenever that base's own requirements allow it.
If the base is an unprotected feature branch, a newly opened stacked PR can merge
there immediately even while checks are queued, moving the parent PR's head and
combining the changes. Before arming auto-merge, confirm that the PR targets the
protected default branch. Keep a stacked child open without auto-merge until its
parent lands, then retarget the child to the default branch and arm auto-merge.

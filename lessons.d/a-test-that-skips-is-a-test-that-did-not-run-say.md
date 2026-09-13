# A test that skips is a test that did not run — say so where it was supposed to run

Both new symlink suites probe the filesystem and skip when it cannot create links. Correct
on Windows. On the Linux runner the probe will succeed — but **nothing asserted that it
had**. A runner image that broke symlink creation would silently stop exercising the
security tests, and the step would stay green.

That is `compiled 0, skipped 1, failed 0` wearing a different hat, and it would have
shipped in the same pull request that removes the original.

**The fix is exactly the `--strict` shape, applied to the test suite instead of the gate:**
an environment variable the CI job sets (`REQUIRE_SYMLINK_TESTS=1`) that turns the skip
into a failure. Local runs keep the honest skip; the machine where the capability must
exist asserts that it does.

Generalised: *a conditional skip needs a caller who can say "not here it isn't."* Any
`skipTest`/`SKIP` guarding a capability that is mandatory somewhere should have a way for
that somewhere to demand it. Verify both directions — the skip path locally, and the fatal
path by setting the variable on a box that genuinely lacks the capability, which is the
one place you can observe the failure for free.

---
category: BUILD files & dependency management
---

# build-tool -dry-run does not run BUILD validation; use -validate-build-files -emit-plan <file> to reproduce 'undeclared local package refs' locally

I checked a new Perl `BUILD_windows` with `build-tool -dry-run -validate-build-files` and saw no error, pushed, and the Windows build job then failed with `undeclared local package refs: perl/hmac, ...` (pbkdf2 referenced sibling libs its `cpanfile` did not declare). `-dry-run` prints the build report and skips the validation step, so silence proved nothing.

Fix: declare the dependency in the language's metadata file (`cpanfile` `requires 'CodingAdventures::HMAC'`) and make the Unix BUILD consistent (`cpanm --notest --quiet Test2::V0` plus a sibling `PERL5LIB`, not `--installdeps`, which would try to fetch the local module from CPAN).

Next time: reproduce validation locally with `build-tool -root . -validate-build-files -emit-plan /tmp/plan.json` and grep for `validation failed`. Prove the check can fail by reverting the fix once and confirming it reports the error.

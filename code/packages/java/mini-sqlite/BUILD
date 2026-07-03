(cd ../sql-backend && gradle jar --quiet) || true
(cd ../sql-planner && gradle jar --quiet)
(cd ../sql-optimizer && gradle jar --quiet)
(cd ../sql-codegen && gradle jar --quiet)
(cd ../sql-vm && gradle jar --quiet)
gradle test jacocoTestReport jacocoTestCoverageVerification

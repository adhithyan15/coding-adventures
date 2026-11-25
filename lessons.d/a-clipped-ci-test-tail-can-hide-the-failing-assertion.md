# A clipped CI test tail can hide the failing assertion

When a repeated test command fails, later output can push the original assertion
outside a fixed-size tail. Keep concise summaries for normal review, but print the
complete captured log on failure so the retained job output contains the cause.
Preserve the original test exit codes and gates; better evidence does not establish
whether a failure is an environmental flake.

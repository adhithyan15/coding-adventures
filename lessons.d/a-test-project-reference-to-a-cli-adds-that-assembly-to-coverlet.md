---
category: Testing & coverage
---

# A test project reference to a CLI adds that assembly to Coverlet coverage

Adding a test reference to the executable project makes its command entry point
part of the instrumented coverage run. The existing library tests may all pass
while the package BUILD coverage threshold fails because CLI parsing and error
paths were not exercised. Test the actual entry point for help, invalid input,
revision selection, output failures, and a report that replays through the
public reader; then rerun the package's configured coverage threshold.

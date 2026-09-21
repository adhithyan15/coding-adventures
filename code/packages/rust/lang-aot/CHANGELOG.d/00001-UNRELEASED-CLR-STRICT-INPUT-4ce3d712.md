## Unreleased — strict CLR encoded integer input (CLR13)

Execute the opt-in strict scalar artifact path with full-width signed integer
input and non-consuming availability peeks. The integration proof lowers exact
MemberRef host calls, runs them through `clr-simulator`, and observes sequential
i64 reads plus normalized zero/one peek results. Default source routing and its
encoded input refusals remain unchanged.

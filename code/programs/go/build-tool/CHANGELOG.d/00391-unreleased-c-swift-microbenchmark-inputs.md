# Unreleased — C and Swift benchmark fixture consumers

Select the C and Swift statistics and microbenchmark packages when the exact
canonical paired-results fixture changes. Export that file to Kiln and include
it in each package's legacy fallback hash. Sixteen implemented consumers now
share selection, language filtering, missing-consumer and cache-invalidation
checks; future language ports remain unregistered until they have actual readers.

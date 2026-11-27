---
category: Ruby
---

# Ruby hash literals must follow StandardRB spacing in test fixtures

StandardRB disallows spaces immediately inside single-line hash literals in
this repository. Environment fixtures written as `{ "KEY" => "value" }` fail
the package's `standardrb --no-fix` BUILD step. Follow the existing compact
form (`{"KEY" => "value"}`) and rerun StandardRB before committing.

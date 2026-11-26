---
category: Ruby
---

# The package BUILD's pinned Bundler can expose failures that global Ruby gems hide

A Ruby package can pass when run against globally installed gems and still fail
under the dependency versions resolved by its Gemfile. The Ruby microbenchmark
lane initially passed with global JSON 2.9 and SimpleCov 1.3, but its package
Bundler selected JSON 3.2 and SimpleCov 0.22. JSON 3.2 rejects the old
`create_additions:` option; SimpleCov 0.22 also exposed that gemspec evaluation
had loaded the static `version.rb` before coverage instrumentation, leaving
those tracked lines uncovered and failing the library's coverage threshold.

The fix was to use JSON's default non-additions parsing, keep `allow_nan: false`
where report validation needs it, and filter the generated version constants
from the coverage target because Bundler evaluates the gemspec before the test
helper starts SimpleCov. The production logic itself did not change.

Before treating a Ruby package as validated, install and run its dependencies
through the package Gemfile, then run its BUILD checks under that bundle. When
local or global Bundler configuration is unsuitable, isolate the run without
changing the user's configuration:

 ```powershell
 $env:BUNDLE_IGNORE_CONFIG = '1'
 $env:BUNDLE_PATH = Join-Path $env:TEMP 'task-specific-ruby-bundle'
 bundle install
 bundle exec ruby -S standardrb --no-fix lib/ test/
 bundle exec ruby -S rake test
 ```

Do not infer pinned-environment correctness from a direct `ruby` or global
`standardrb` invocation. The global gem versions can mask API drift and
coverage-ordering failures that the package BUILD actually exercises.

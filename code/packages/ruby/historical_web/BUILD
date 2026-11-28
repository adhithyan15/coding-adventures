#!/usr/bin/env bash
# build-tool: deps=ruby/test_bootstrap
set -euo pipefail
bundle config set path vendor/bundle
bundle install --quiet
bundle exec standardrb --no-fix lib/ test/
bundle exec rake test

cd ../rank_statistics && bundle config set path vendor/bundle && bundle install --quiet --full-index && bundle exec rake test
cd ../microbenchmark && bundle config set path vendor/bundle && bundle install --quiet --full-index
bundle exec standardrb --no-fix lib/ test/ bin/
bundle exec rake test
gem build coding_adventures_microbenchmark.gemspec

cd ../../../packages/ruby/irc_proto && bundle install --quiet
cd ../../../packages/ruby/irc_framing && bundle install --quiet
cd ../../../packages/ruby/irc_server && bundle install --quiet
cd ../../../packages/ruby/irc_net_stdlib && bundle install --quiet
bundle install --quiet
bundle exec ruby -Ilib -e "require 'ircd'; puts 'ircd loads OK'"
bundle exec rake test

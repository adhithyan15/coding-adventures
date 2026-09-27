mix deps.get --quiet
mix format --check-formatted "lib/**/*.{ex,exs}" "test/**/*.{ex,exs}" mix.exs
MIX_ENV=test mix compile --warnings-as-errors
mix test --cover

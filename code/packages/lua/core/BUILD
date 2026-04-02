(cd ../cpu_pipeline && luarocks make --local --no-manifest --no-deps coding-adventures-cpu-pipeline-0.1.0-1.rockspec)
(cd ../cpu_simulator && luarocks make --local --no-manifest --no-deps coding-adventures-cpu-simulator-0.1.0-1.rockspec)
luarocks make --local --no-manifest --no-deps coding-adventures-core-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_

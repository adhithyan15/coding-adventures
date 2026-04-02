(cd ../cpu_pipeline && luarocks make --local --force --no-manifest coding-adventures-cpu-pipeline-0.1.0-1.rockspec)
(cd ../cpu_simulator && luarocks make --local --force --no-manifest coding-adventures-cpu-simulator-0.1.0-1.rockspec)
luarocks make --local --deps-mode none coding-adventures-core-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_

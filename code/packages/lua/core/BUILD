luarocks show coding-adventures-cpu-pipeline >/dev/null 2>&1 || (cd ../cpu_pipeline && luarocks make --local coding-adventures-cpu-pipeline-0.1.0-1.rockspec)
luarocks show coding-adventures-cpu-simulator >/dev/null 2>&1 || (cd ../cpu_simulator && luarocks make --local coding-adventures-cpu-simulator-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-core-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_

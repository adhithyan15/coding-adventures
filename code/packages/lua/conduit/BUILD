cd ext/conduit_native && cargo build --release && cd ../..
python3 -c "import glob,shutil,sys,os; libs=glob.glob('ext/conduit_native/target/release/libconduit_native.*'); src=next((l for l in libs if l.endswith(('.so','.dylib','.dll'))),None); sys.exit('no libconduit_native found') if not src else shutil.copy(src,'conduit/conduit_native.dll' if os.name=='nt' else 'conduit/conduit_native.so')"
luarocks install luasocket 2>/dev/null || true
cd tests && busted . --verbose --pattern=test_

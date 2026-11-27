#!/bin/sh
set -eu

self=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
if ! repo=$(git -C "$self" rev-parse --show-toplevel 2>/dev/null); then
    echo "Git repository root not found for $self" >&2
    exit 1
fi

license_blob=4a4dc9a35858c625d7a0833d0a63bb2a7e05563c
actual_license_blob=$(git -C "$repo" hash-object --no-filters "$self/LICENSE")
if [ "$actual_license_blob" != "$license_blob" ]; then
    echo "Package LICENSE does not match the pinned repository license blob" >&2
    exit 1
fi

tmp_parent=$(CDPATH= cd -- "${TMPDIR:-/tmp}" && pwd)
run_dir=$(mktemp -d "$tmp_parent/ca-heap-installed.XXXXXX")
case "$run_dir/" in
    "$repo/"*|"$self/"*)
        echo "Installed smoke temp directory must be outside the repository: $run_dir" >&2
        exit 1
        ;;
esac
prefix="$run_dir/install-prefix"
relocated="$run_dir/relocated-prefix"
unavailable="$run_dir/original-prefix-temporarily-unavailable"
package_build="$run_dir/package-build"
consumer_source="$run_dir/consumer-source"
consumer_build="$run_dir/consumer-build"
log="$run_dir/commands.log"
mkdir -p "$consumer_source"
cp "$self/tests/installed/CMakeLists.txt" "$consumer_source/"
cp "$self/tests/installed/main.cpp" "$consumer_source/"

restore_original() {
    if [ -d "$unavailable" ] && [ ! -e "$prefix" ]; then
        mv -- "$unavailable" "$prefix"
    fi
}
trap restore_original EXIT HUP INT TERM

run_checked() {
    printf '\n$' >> "$log"
    printf ' %s' "$@" >> "$log"
    printf '\n' >> "$log"
    if "$@" >> "$log" 2>&1; then
        return 0
    else
        code=$?
    fi
    cat "$log" >&2
    echo "Command failed with exit $code; full output: $log" >&2
    return "$code"
}

run_checked cmake -S "$self" -B "$package_build" \
    "-DCMAKE_INSTALL_PREFIX=$prefix" \
    -DCMAKE_FIND_PACKAGE_NO_PACKAGE_REGISTRY=ON \
    -DCMAKE_FIND_PACKAGE_NO_SYSTEM_PACKAGE_REGISTRY=ON
run_checked cmake --build "$package_build"
run_checked cmake --install "$package_build"

installed_license="$prefix/share/licenses/ca_heap/LICENSE"
if [ ! -f "$installed_license" ]; then
    echo "Installed LICENSE is missing: $installed_license" >&2
    exit 1
fi
installed_license_blob=$(git -C "$repo" hash-object --no-filters "$installed_license")
if [ "$installed_license_blob" != "$license_blob" ] || ! cmp -s "$self/LICENSE" "$installed_license"; then
    echo "Installed LICENSE differs from the pinned package license" >&2
    exit 1
fi

cp -a "$prefix" "$relocated"
mv -- "$prefix" "$unavailable"

old_prefix=${CMAKE_PREFIX_PATH-}
old_module=${CMAKE_MODULE_PATH-}
old_find_root=${CMAKE_FIND_ROOT_PATH-}
old_toolchain=${CMAKE_TOOLCHAIN_FILE-}
unset CMAKE_PREFIX_PATH CMAKE_MODULE_PATH CMAKE_FIND_ROOT_PATH CMAKE_TOOLCHAIN_FILE
restore_cmake_env() {
    CMAKE_PREFIX_PATH=$old_prefix; export CMAKE_PREFIX_PATH
    CMAKE_MODULE_PATH=$old_module; export CMAKE_MODULE_PATH
    CMAKE_FIND_ROOT_PATH=$old_find_root; export CMAKE_FIND_ROOT_PATH
    CMAKE_TOOLCHAIN_FILE=$old_toolchain; export CMAKE_TOOLCHAIN_FILE
}
trap 'restore_original; restore_cmake_env' EXIT HUP INT TERM

run_checked cmake -S "$consumer_source" -B "$consumer_build" \
    "-DCA_HEAP_RELOCATED_PREFIX=$relocated" \
    "-DCA_HEAP_SOURCE_ROOT=$self" \
    "-DCA_HEAP_ORIGINAL_PREFIX=$prefix" \
    -DCMAKE_PREFIX_PATH= \
    -DCMAKE_FIND_PACKAGE_NO_PACKAGE_REGISTRY=ON \
    -DCMAKE_FIND_PACKAGE_NO_SYSTEM_PACKAGE_REGISTRY=ON \
    -DCMAKE_FIND_USE_PACKAGE_REGISTRY=OFF \
    -DCMAKE_FIND_USE_SYSTEM_PACKAGE_REGISTRY=OFF
run_checked cmake --build "$consumer_build"
run_checked cmake -E chdir "$consumer_build" ctest -C Release --output-on-failure

restore_original
trap - EXIT HUP INT TERM
restore_cmake_env
printf 'Installed-prefix consumer passed. Evidence: %s\n' "$run_dir"

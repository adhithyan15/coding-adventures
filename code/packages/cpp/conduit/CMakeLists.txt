cmake_minimum_required(VERSION 3.20)
project(Conduit VERSION 0.1.0
        DESCRIPTION "Sinatra-style web framework for C++ over the Conduit C ABI"
        LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# The wrapper itself is header-only.
add_library(Conduit INTERFACE)
add_library(Conduit::Conduit ALIAS Conduit)
target_include_directories(Conduit INTERFACE
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../../rust/conduit-capi/include>
    $<INSTALL_INTERFACE:include>)

# Link against the prebuilt Rust C ABI static library. Build it first with:
#   ( cd ../../rust/conduit-capi && cargo build --release )
# (the canonical CI path is `sh tools/run-tests.sh` on Unix and
# `tools\run-tests.ps1` on Windows; both do this for you).
#
# The archive's name and location depend on the platform's Rust target:
#
#   Linux / macOS            target/release/libconduit_capi.a
#   Windows, MSVC (default)  target/release/conduit_capi.lib
#   Windows, MinGW           target/x86_64-pc-windows-gnu/release/libconduit_capi.a
#                            (needs `cargo ... --target x86_64-pc-windows-gnu`)
#
# A Rust staticlib does not carry the system libraries its standard library
# calls, so every consumer must name them (on Unix that is left to the
# compiler driver; `rustc --print native-static-libs` lists them). On Windows
# they are:
#   kernel32 advapi32 ntdll userenv bcrypt ws2_32  - process, registry, RNG, sockets
#   synchronization                                 - WaitOnAddress / WakeByAddress*
#                                                     (std's parking primitives)
set(CONDUIT_RUST_TARGET_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../rust/target")
if(MSVC)
    set(CONDUIT_CAPI_LIB "${CONDUIT_RUST_TARGET_DIR}/release/conduit_capi.lib")
    set(CONDUIT_CAPI_SYSTEM_LIBS kernel32 advapi32 ntdll userenv bcrypt ws2_32 synchronization)
elseif(WIN32)
    set(CONDUIT_CAPI_LIB
        "${CONDUIT_RUST_TARGET_DIR}/x86_64-pc-windows-gnu/release/libconduit_capi.a")
    set(CONDUIT_CAPI_SYSTEM_LIBS ws2_32 userenv bcrypt ntdll advapi32 dbghelp kernel32
                                 synchronization gcc_eh)
else()
    set(CONDUIT_CAPI_LIB "${CONDUIT_RUST_TARGET_DIR}/release/libconduit_capi.a")
    set(CONDUIT_CAPI_SYSTEM_LIBS "")
endif()
target_link_libraries(Conduit INTERFACE ${CONDUIT_CAPI_LIB} ${CONDUIT_CAPI_SYSTEM_LIBS})

if(MSVC)
    target_compile_options(Conduit INTERFACE /W4)
else()
    target_compile_options(Conduit INTERFACE -Wall -Wextra -Wpedantic)
endif()

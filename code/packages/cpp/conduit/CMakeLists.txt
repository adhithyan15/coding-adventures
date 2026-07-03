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
# (the canonical CI path is `sh tools/run-tests.sh`, which does this for you).
set(CONDUIT_CAPI_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../../rust/target/release/libconduit_capi.a")
target_link_libraries(Conduit INTERFACE ${CONDUIT_CAPI_LIB})

if(MSVC)
    target_compile_options(Conduit INTERFACE /W4)
else()
    target_compile_options(Conduit INTERFACE -Wall -Wextra -Wpedantic)
endif()

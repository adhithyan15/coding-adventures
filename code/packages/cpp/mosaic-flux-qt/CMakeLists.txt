# CMakeLists.txt — Mosaic.Flux for Qt / C++
#
# Header-only library, so the main target is INTERFACE.  Tests
# (under tests/) consume that target via add_subdirectory.
#
# Build + run tests:
#   cmake -B build -S .
#   cmake --build build
#   ctest --test-dir build --output-on-failure

cmake_minimum_required(VERSION 3.20)
project(MosaicFlux
        VERSION 0.1.0
        DESCRIPTION "Strict-Flux runtime for Mosaic UI's Qt emitter"
        LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# The library: pure header-only INTERFACE target.  Consumers
# `target_link_libraries(... PRIVATE MosaicFlux::MosaicFlux)` and
# get the include directory automatically.
add_library(MosaicFlux INTERFACE)
add_library(MosaicFlux::MosaicFlux ALIAS MosaicFlux)

target_include_directories(MosaicFlux INTERFACE
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>)

if(MSVC)
    target_compile_options(MosaicFlux INTERFACE /W4 /WX)
else()
    target_compile_options(MosaicFlux INTERFACE
        -Wall -Wextra -Wpedantic -Werror)
endif()

enable_testing()
add_subdirectory(tests)

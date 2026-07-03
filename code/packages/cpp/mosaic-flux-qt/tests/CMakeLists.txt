# tests/CMakeLists.txt — register each test_*.cpp as an executable
# linked against MosaicFlux::MosaicFlux, then register it with
# ctest.  No GTest / Catch2 dependency: we use a tiny hand-rolled
# assertion macro (see test_harness.h) to keep the package zero-dep.

set(MOSAICFLUX_TESTS
    test_action
    test_store
    test_middleware
    test_selector
    test_devtools)

foreach(test ${MOSAICFLUX_TESTS})
    add_executable(${test} ${test}.cpp)
    target_link_libraries(${test} PRIVATE MosaicFlux::MosaicFlux)
    add_test(NAME ${test} COMMAND ${test})
endforeach()

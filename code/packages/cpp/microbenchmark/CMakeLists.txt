cmake_minimum_required(VERSION 3.16)
project(ca_microbenchmark VERSION 0.1.0 LANGUAGES CXX)
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

find_package(ca_bounded_json CONFIG REQUIRED)
find_package(ca_rank_statistics CONFIG REQUIRED)

add_library(ca_microbenchmark STATIC src/microbenchmark.cpp src/array_workload.cpp)
set_target_properties(ca_microbenchmark PROPERTIES EXPORT_NAME microbenchmark)
add_library(ca::microbenchmark ALIAS ca_microbenchmark)
target_compile_features(ca_microbenchmark PUBLIC cxx_std_17)
target_include_directories(ca_microbenchmark PUBLIC
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>)
target_link_libraries(ca_microbenchmark PUBLIC ca::bounded_json ca::rank_statistics)

add_executable(array-microbenchmark src/array_driver.cpp)
target_link_libraries(array-microbenchmark PRIVATE ca::microbenchmark)

enable_testing()
add_executable(microbenchmark-tests tests/microbenchmark_test.cpp)
target_include_directories(microbenchmark-tests PRIVATE
  ../bounded-json/include ../rank-statistics/include ../../c/iso-harness/include)
target_link_libraries(microbenchmark-tests PRIVATE ca::microbenchmark)
add_test(NAME microbenchmark-tests COMMAND microbenchmark-tests)
set_tests_properties(microbenchmark-tests PROPERTIES WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})
add_test(NAME array-microbenchmark-smoke
  COMMAND array-microbenchmark --pairs 40 --target-ns 1000 --revision cpp-smoke)

install(TARGETS ca_microbenchmark array-microbenchmark EXPORT ca_microbenchmark_targets
  ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
  RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
install(DIRECTORY include/ DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(EXPORT ca_microbenchmark_targets NAMESPACE ca::
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_microbenchmark)
configure_package_config_file(cmake/ca_microbenchmarkConfig.cmake.in
  ${CMAKE_CURRENT_BINARY_DIR}/ca_microbenchmarkConfig.cmake
  INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_microbenchmark)
write_basic_package_version_file(
  ${CMAKE_CURRENT_BINARY_DIR}/ca_microbenchmarkConfigVersion.cmake
  VERSION ${PROJECT_VERSION} COMPATIBILITY SameMajorVersion)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/ca_microbenchmarkConfig.cmake
  ${CMAKE_CURRENT_BINARY_DIR}/ca_microbenchmarkConfigVersion.cmake
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_microbenchmark)

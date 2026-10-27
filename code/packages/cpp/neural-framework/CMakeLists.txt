cmake_minimum_required(VERSION 3.16)
project(ca_neural_framework VERSION 0.1.0 LANGUAGES CXX)
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)
add_library(ca_neural_framework INTERFACE)
set_target_properties(ca_neural_framework PROPERTIES EXPORT_NAME neural_framework)
add_library(ca::neural_framework ALIAS ca_neural_framework)
target_compile_features(ca_neural_framework INTERFACE cxx_std_17)
target_include_directories(ca_neural_framework INTERFACE
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../matrix/include>
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../float-math/include>
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../bounded-json/include>
  $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>)
install(TARGETS ca_neural_framework EXPORT ca_neural_framework_targets)
# The exported target is self-contained: all three native public dependencies
# are header-only; install the actual owned payload, not source-tree references.
install(FILES ../matrix/include/matrix.hpp ../float-math/include/float_math.hpp
  ../bounded-json/include/bounded_json.hpp include/neural_framework.hpp
  DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(EXPORT ca_neural_framework_targets NAMESPACE ca::
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_neural_framework)
configure_package_config_file(cmake/ca_neural_frameworkConfig.cmake.in
  ${CMAKE_CURRENT_BINARY_DIR}/ca_neural_frameworkConfig.cmake
  INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_neural_framework)
write_basic_package_version_file(
  ${CMAKE_CURRENT_BINARY_DIR}/ca_neural_frameworkConfigVersion.cmake
  VERSION ${PROJECT_VERSION} COMPATIBILITY SameMajorVersion)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/ca_neural_frameworkConfig.cmake
  ${CMAKE_CURRENT_BINARY_DIR}/ca_neural_frameworkConfigVersion.cmake
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_neural_framework)

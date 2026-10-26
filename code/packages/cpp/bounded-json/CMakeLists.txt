cmake_minimum_required(VERSION 3.16)
project(ca_bounded_json VERSION 0.1.0 LANGUAGES CXX)
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)
add_library(ca_bounded_json INTERFACE)
set_target_properties(ca_bounded_json PROPERTIES EXPORT_NAME bounded_json)
add_library(ca::bounded_json ALIAS ca_bounded_json)
target_compile_features(ca_bounded_json INTERFACE cxx_std_17)
target_include_directories(ca_bounded_json INTERFACE
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>)
install(TARGETS ca_bounded_json EXPORT ca_bounded_json_targets)
install(FILES include/bounded_json.hpp DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(EXPORT ca_bounded_json_targets NAMESPACE ca::
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_bounded_json)
configure_package_config_file(cmake/ca_bounded_jsonConfig.cmake.in
  ${CMAKE_CURRENT_BINARY_DIR}/ca_bounded_jsonConfig.cmake
  INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_bounded_json)
write_basic_package_version_file(
  ${CMAKE_CURRENT_BINARY_DIR}/ca_bounded_jsonConfigVersion.cmake
  VERSION ${PROJECT_VERSION} COMPATIBILITY SameMajorVersion)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/ca_bounded_jsonConfig.cmake
  ${CMAKE_CURRENT_BINARY_DIR}/ca_bounded_jsonConfigVersion.cmake
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_bounded_json)

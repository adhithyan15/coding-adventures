cmake_minimum_required(VERSION 3.16)
project(ca_heap VERSION 0.2.0 LANGUAGES CXX)

include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

add_library(ca_heap INTERFACE)
set_target_properties(ca_heap PROPERTIES EXPORT_NAME heap)
add_library(ca::heap ALIAS ca_heap)

target_compile_features(ca_heap INTERFACE cxx_std_17)
target_include_directories(ca_heap INTERFACE
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>)

install(TARGETS ca_heap EXPORT ca_heap_targets)
install(FILES include/heap.hpp DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(FILES LICENSE DESTINATION ${CMAKE_INSTALL_DATADIR}/licenses/ca_heap)
install(EXPORT ca_heap_targets NAMESPACE ca::
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_heap)

configure_package_config_file(cmake/ca_heapConfig.cmake.in
  ${CMAKE_CURRENT_BINARY_DIR}/ca_heapConfig.cmake
  INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_heap)
write_basic_package_version_file(
  ${CMAKE_CURRENT_BINARY_DIR}/ca_heapConfigVersion.cmake
  VERSION ${PROJECT_VERSION} COMPATIBILITY SameMajorVersion ARCH_INDEPENDENT)
install(FILES
  ${CMAKE_CURRENT_BINARY_DIR}/ca_heapConfig.cmake
  ${CMAKE_CURRENT_BINARY_DIR}/ca_heapConfigVersion.cmake
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_heap)

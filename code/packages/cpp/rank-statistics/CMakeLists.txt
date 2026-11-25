cmake_minimum_required(VERSION 3.16)
project(ca_rank_statistics VERSION 0.1.0 LANGUAGES CXX)
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

add_library(ca_rank_statistics INTERFACE)
set_target_properties(ca_rank_statistics PROPERTIES EXPORT_NAME rank_statistics)
add_library(ca::rank_statistics ALIAS ca_rank_statistics)
target_compile_features(ca_rank_statistics INTERFACE cxx_std_17)
target_include_directories(ca_rank_statistics INTERFACE
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>)

install(TARGETS ca_rank_statistics EXPORT ca_rank_statistics_targets)
install(DIRECTORY include/ DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(EXPORT ca_rank_statistics_targets NAMESPACE ca::
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_rank_statistics)
configure_package_config_file(cmake/ca_rank_statisticsConfig.cmake.in
  ${CMAKE_CURRENT_BINARY_DIR}/ca_rank_statisticsConfig.cmake
  INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_rank_statistics)
write_basic_package_version_file(
  ${CMAKE_CURRENT_BINARY_DIR}/ca_rank_statisticsConfigVersion.cmake
  VERSION ${PROJECT_VERSION} COMPATIBILITY SameMajorVersion)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/ca_rank_statisticsConfig.cmake
  ${CMAKE_CURRENT_BINARY_DIR}/ca_rank_statisticsConfigVersion.cmake
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ca_rank_statistics)

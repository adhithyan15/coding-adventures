cmake_minimum_required(VERSION 3.20)
project(coding_adventures_bounded_json VERSION 0.1.0 LANGUAGES C)
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)
add_library(bounded_json STATIC src/bounded_json.c)
add_library(coding_adventures::bounded_json ALIAS bounded_json)
set_target_properties(bounded_json PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)
target_include_directories(bounded_json PUBLIC
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>)
if(MSVC)
  target_compile_options(bounded_json PRIVATE /W4 /WX /permissive-)
else()
  target_compile_options(bounded_json PRIVATE -Wall -Wextra -Werror -pedantic-errors)
endif()
install(TARGETS bounded_json EXPORT bounded_json_targets ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})
install(FILES include/bounded_json.h DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(FILES README.md CHANGELOG.md LICENSE required_capabilities.json
  DESTINATION ${CMAKE_INSTALL_DATADIR}/doc/coding_adventures_bounded_json)
install(EXPORT bounded_json_targets NAMESPACE coding_adventures::
  FILE bounded_json-targets.cmake DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/bounded_json)
configure_package_config_file(cmake/bounded_json-config.cmake.in
  ${CMAKE_CURRENT_BINARY_DIR}/bounded_json-config.cmake
  INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/bounded_json)
write_basic_package_version_file(${CMAKE_CURRENT_BINARY_DIR}/bounded_json-config-version.cmake
  VERSION ${PROJECT_VERSION} COMPATIBILITY SameMajorVersion)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/bounded_json-config.cmake
  ${CMAKE_CURRENT_BINARY_DIR}/bounded_json-config-version.cmake
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/bounded_json)

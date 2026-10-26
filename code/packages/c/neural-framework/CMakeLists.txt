cmake_minimum_required(VERSION 3.20)
project(coding_adventures_neural_framework VERSION 0.1.0 LANGUAGES C)
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)
# The unchanged native dependency translation units are separate archive members.
# Installation is self-contained; consumers need neither this checkout nor a
# runtime package manager. No generated/downloaded numerical implementation.
add_library(neural_framework STATIC src/neural_framework.c
  ../matrix/src/matrix.c ../float-math/src/float_math.c
  ../bounded-json/src/bounded_json.c)
add_library(coding_adventures::neural_framework ALIAS neural_framework)
set_target_properties(neural_framework PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)
target_include_directories(neural_framework PUBLIC
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../matrix/include>
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../float-math/include>
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../bounded-json/include>
  $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/coding_adventures/neural_framework>)
if(MSVC)
  target_compile_options(neural_framework PRIVATE /W4 /WX /permissive-)
  if(CMAKE_C_COMPILER_ID STREQUAL "Clang")
    target_compile_options(neural_framework PRIVATE /clang:-pedantic-errors)
  endif()
else()
  target_compile_options(neural_framework PRIVATE -Wall -Wextra -Werror -pedantic-errors)
endif()
add_executable(neural_framework_xor examples/xor.c)
target_link_libraries(neural_framework_xor PRIVATE neural_framework)
set_target_properties(neural_framework_xor PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)
install(TARGETS neural_framework EXPORT neural_framework_targets ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})
install(FILES include/neural_framework.h ../matrix/include/matrix.h
  ../float-math/include/float_math.h ../bounded-json/include/bounded_json.h
  DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/coding_adventures/neural_framework)
install(FILES README.md CHANGELOG.md LICENSE required_capabilities.json
  DESTINATION ${CMAKE_INSTALL_DATADIR}/doc/coding_adventures_neural_framework)
install(FILES ../matrix/README.md ../matrix/CHANGELOG.md
  DESTINATION ${CMAKE_INSTALL_DATADIR}/doc/coding_adventures_neural_framework/dependencies/matrix)
install(FILES ../float-math/README.md ../float-math/CHANGELOG.md
  DESTINATION ${CMAKE_INSTALL_DATADIR}/doc/coding_adventures_neural_framework/dependencies/float-math)
install(FILES ../bounded-json/README.md ../bounded-json/CHANGELOG.md ../bounded-json/LICENSE
  DESTINATION ${CMAKE_INSTALL_DATADIR}/doc/coding_adventures_neural_framework/dependencies/bounded-json)
install(EXPORT neural_framework_targets NAMESPACE coding_adventures::
  FILE neural_framework-targets.cmake DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/neural_framework)
configure_package_config_file(cmake/neural_framework-config.cmake.in
  ${CMAKE_CURRENT_BINARY_DIR}/neural_framework-config.cmake
  INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/neural_framework)
write_basic_package_version_file(${CMAKE_CURRENT_BINARY_DIR}/neural_framework-config-version.cmake
  VERSION ${PROJECT_VERSION} COMPATIBILITY SameMajorVersion)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/neural_framework-config.cmake
  ${CMAKE_CURRENT_BINARY_DIR}/neural_framework-config-version.cmake
  DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/neural_framework)

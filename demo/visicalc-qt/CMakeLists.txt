# CMakeLists.txt — C++/CMake host for VC2-qt, computing on the shared Rust
# spreadsheet engine through its C ABI.
#
# Two ways to build the demo:
#
#   1. qmake (no CMake needed):
#        bash scripts/build.sh        # regenerate QML + build & vendor the engine
#        qmake && make && ./visicalc_qt_app
#
#   2. CMake:
#        bash scripts/build.sh
#        cmake -B build-cmake && cmake --build build-cmake && ./build-cmake/visicalc_qt_app
#
# Both compile the same sources (src/main.cpp + src/SpreadsheetModel.cpp) and
# link the vendored Rust engine static library (Vendor/libspreadsheet_capi.a,
# produced by scripts/build.sh). The model exposes the engine to QML as the
# `model` context property; main.qml's generated Grid / FormulaBar bind to it.

cmake_minimum_required(VERSION 3.16)
project(visicalc_qt LANGUAGES CXX)

set(CMAKE_AUTOMOC ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Qt 6 — adjust the find_package version if you have Qt 5.
# QuickControls2 is needed for QQuickStyle (main.cpp forces the Basic style so
# the demo's custom Button chrome in InfiniteSheet.qml renders).
find_package(Qt6 6.2 COMPONENTS Core Gui Qml Quick QuickControls2 REQUIRED)

add_executable(visicalc_qt_app
    src/main.cpp
    src/SpreadsheetModel.cpp
)

# main.cpp needs the directory holding main.qml + the qml/ module so it can set
# the QML import path and load the UI regardless of the working directory.
target_compile_definitions(visicalc_qt_app PRIVATE
    VISICALC_QML_DIR="${CMAKE_CURRENT_SOURCE_DIR}"
)

# The model header (src/) and the C ABI header (Vendor/spreadsheet.h).
target_include_directories(visicalc_qt_app PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src
    ${CMAKE_CURRENT_SOURCE_DIR}/Vendor
)

target_link_libraries(visicalc_qt_app
    PRIVATE Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick Qt6::QuickControls2
)

# Link the Rust engine's C ABI static library, vendored by scripts/build.sh.
target_link_libraries(visicalc_qt_app
    PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/Vendor/libspreadsheet_capi.a
)
# Rust's std pulls in a few system libraries through the static archive.
if(APPLE)
    target_link_libraries(visicalc_qt_app PRIVATE
        "-framework CoreFoundation" "-framework Security" "-framework SystemConfiguration")
elseif(UNIX)
    target_link_libraries(visicalc_qt_app PRIVATE dl pthread m)
endif()

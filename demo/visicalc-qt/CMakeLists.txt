# CMakeLists.txt — optional C++/CMake host for VC2-qt.
#
# Two ways to run the demo:
#
#   1. Quickest:   `qml main.qml`  (requires Qt 6's `qml` runner)
#   2. CMake build: `cmake -B build && cmake --build build && ./build/visicalc_qt_app`
#
# The CMake path is needed when you want to ship a real binary
# (Qt's `qml` runner is for development). It builds a tiny C++
# wrapper that just hosts main.qml in a QQmlApplicationEngine.

cmake_minimum_required(VERSION 3.16)
project(visicalc_qt LANGUAGES CXX)

set(CMAKE_AUTOMOC ON)
set(CMAKE_CXX_STANDARD 17)

# Qt 6 — adjust the find_package version if you have Qt 5.
find_package(Qt6 6.2 COMPONENTS Core Gui Qml Quick REQUIRED)

add_executable(visicalc_qt_app
    src/main.cpp
)

target_link_libraries(visicalc_qt_app
    PRIVATE Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick
)

# Bundle the QML sources via Qt's resource system. main.cpp loads
# `qrc:/main.qml`; the QML import path is set to look up the
# FormulaBar module alongside.
qt_add_qml_module(visicalc_qt_app
    URI VisiCalc
    VERSION 1.0
    QML_FILES
        main.qml
        # qmldir is in qml/ (tracked); FormulaBar.qml in build/
        # (auto-generated, gitignored). Both bundled into qrc.
        qml/qmldir
        build/FormulaBar.qml
)

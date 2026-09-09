# Builds the emitted Qt host headlessly. Qt Core only -- MosaicHost depends on
# QLibrary/QObject/QVariant and nothing from Quick or QML, so this needs no
# display and no QML runtime.
cmake_minimum_required(VERSION 3.19)
project(mosaic_qt_effect_driver LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
find_package(Qt6 REQUIRED COMPONENTS Core)
add_executable(mosaic_qt_effect_driver main.cpp MosaicHost.cpp MosaicHost.h)
target_link_libraries(mosaic_qt_effect_driver PRIVATE Qt6::Core)

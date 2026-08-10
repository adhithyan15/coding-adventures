cmake_minimum_required(VERSION 3.21)
project(MosaicQtRuntimeConformance LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 6.7 REQUIRED COMPONENTS Core)

add_executable(mosaic_qt_runtime_conformance
  main.cpp
  MosaicHost.cpp
  MosaicHost.h
)
target_link_libraries(mosaic_qt_runtime_conformance PRIVATE Qt6::Core)

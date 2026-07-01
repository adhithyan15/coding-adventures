# tst_model.pro — qmake project for the headless VisiCalc Qt model test.
#
# We ship a qmake project (not just CMake) because the canonical local
# verification path uses qmake: it builds the engine-backed SpreadsheetModel
# plus the QtTest harness, links the vendored Rust engine static library, and
# runs with no GUI/display required — the Qt equivalent of `swift test`.
#
# Build & run:
#   bash ../scripts/build.sh   # regenerate QML + build & vendor the engine
#   cd test && qmake && make && ./tst_model
#
# (CMakeLists.txt builds the same code into the GUI app for those who have
# CMake; this .pro exists so the test runs with only Qt's qmake + a compiler.)

QT       += core testlib
QT       -= gui

CONFIG   += console c++17
CONFIG   -= app_bundle
TEMPLATE  = app
TARGET    = tst_model

# The model under test lives in ../src; the C ABI header is vendored in
# ../Vendor alongside the static library.
INCLUDEPATH += $$PWD/../src $$PWD/../Vendor

SOURCES += \
    tst_model.cpp \
    $$PWD/../src/SpreadsheetModel.cpp

HEADERS += \
    $$PWD/../src/SpreadsheetModel.h

# Link the Rust engine's C ABI static library, vendored by scripts/build.sh.
# On macOS the static lib pulls in a few system frameworks Rust's std needs.
LIBS += -L$$PWD/../Vendor -lspreadsheet_capi
macx: LIBS += -framework CoreFoundation -framework Security -framework SystemConfiguration
unix:!macx: LIBS += -ldl -lpthread -lm

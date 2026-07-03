# tst_window.pro — qmake project for the headless Qt viewport (windowing) test.
# Same shape as tst_model.pro: builds the engine-backed SpreadsheetModel plus the
# QtTest harness and links the vendored Rust engine static library — no GUI.
#
# Build & run:
#   bash ../scripts/build.sh   # regenerate QML + build & vendor the engine
#   cd test && qmake tst_window.pro && make && ./tst_window

QT       += core testlib
QT       -= gui

CONFIG   += console c++17
CONFIG   -= app_bundle
TEMPLATE  = app
TARGET    = tst_window

INCLUDEPATH += $$PWD/../src $$PWD/../Vendor

SOURCES += \
    tst_window.cpp \
    $$PWD/../src/SpreadsheetModel.cpp

HEADERS += \
    $$PWD/../src/SpreadsheetModel.h

LIBS += -L$$PWD/../Vendor -lspreadsheet_capi
macx: LIBS += -framework CoreFoundation -framework Security -framework SystemConfiguration
unix:!macx: LIBS += -ldl -lpthread -lm

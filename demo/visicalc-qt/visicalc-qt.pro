# visicalc-qt.pro — qmake project for the VisiCalc Qt GUI app, computing on the
# shared Rust engine through its C ABI.
#
# A qmake alternative to CMakeLists.txt (same sources, same engine link), so the
# demo builds with only Qt's qmake + a compiler — no CMake needed.
#
# Build & run:
#   bash scripts/build.sh   # regenerate QML + build & vendor the engine
#   qmake && make && ./visicalc_qt_app

# quickcontrols2 is needed for QQuickStyle (main.cpp forces the Basic style so
# the demo's custom Button chrome in InfiniteSheet.qml renders — the macOS
# native style ignores Button background/contentItem customization).
QT       += core gui qml quick quickcontrols2
CONFIG   += c++17
TEMPLATE  = app
TARGET    = visicalc_qt_app

INCLUDEPATH += $$PWD/src $$PWD/Vendor

# main.cpp needs the directory holding main.qml + the qml/ module so it can set
# the QML import path and load the UI regardless of the working directory.
DEFINES += VISICALC_QML_DIR=\\\"$$PWD\\\"

SOURCES += \
    src/main.cpp \
    src/SpreadsheetModel.cpp

HEADERS += \
    src/SpreadsheetModel.h

# The Rust engine's C ABI static library, vendored by scripts/build.sh.
LIBS += -L$$PWD/Vendor -lspreadsheet_capi
macx: LIBS += -framework CoreFoundation -framework Security -framework SystemConfiguration
unix:!macx: LIBS += -ldl -lpthread -lm

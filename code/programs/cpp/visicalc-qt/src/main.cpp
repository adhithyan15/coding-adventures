// main.cpp — C++ host for the VisiCalc Qt demo, now computing on the shared
// Rust spreadsheet engine through its C ABI.
//
// Unlike the `qml main.qml` development runner (which can't expose a C++ object
// or link the engine), this compiled binary:
//   1. creates the engine-backed `SpreadsheetModel`,
//   2. exposes it to QML as the `model` context property, and
//   3. loads main.qml, whose generated Grid / FormulaBar bind to `model`.
//
// So the grid you see is rendered from values the Rust engine computed, and
// editing the formula bar writes through to the engine and recomputes — the
// same one-engine-everywhere architecture the SwiftUI and web demos use.

#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QUrl>
#include <QDir>

#include "SpreadsheetModel.h"

// The directory containing main.qml and the qml/ module (whose qmldir points at
// the generated build/*.qml). Injected by the build so `import "qml"` resolves
// and main.qml loads regardless of the working directory. scripts/build.sh
// regenerates build/*.qml; CMake/qmake define VISICALC_QML_DIR to this folder.
#ifndef VISICALC_QML_DIR
#define VISICALC_QML_DIR "."
#endif

int main(int argc, char *argv[]) {
    QGuiApplication app(argc, argv);

    // Use the Basic Controls style so the demo's custom button styling (rounded
    // chips in InfiniteSheet.qml) is honored — the macOS *native* style ignores
    // Button background/contentItem customization. Must be set before the engine
    // loads any QML.
    QQuickStyle::setStyle(QStringLiteral("Basic"));

    SpreadsheetModel model;

    QQmlApplicationEngine engine;
    // Make the generated qml/ module importable by main.qml's `import "qml"`.
    engine.addImportPath(QStringLiteral(VISICALC_QML_DIR));
    // Expose the engine-backed model to QML before loading the UI.
    engine.rootContext()->setContextProperty(QStringLiteral("model"), &model);

    const QString qmlPath =
        QDir(QStringLiteral(VISICALC_QML_DIR)).filePath(QStringLiteral("main.qml"));
    engine.load(QUrl::fromLocalFile(qmlPath));
    if (engine.rootObjects().isEmpty()) return -1;

    return app.exec();
}

// main.cpp — tiny C++ wrapper around main.qml for the
// CMake-built variant of VC2-qt.
//
// The `qml main.qml` runner is fine for development; this exists
// so `cmake --build && ./visicalc_qt_app` produces a real binary
// you could ship. The wrapper does nothing but load main.qml into
// a standard QQmlApplicationEngine.

#include <QGuiApplication>
#include <QQmlApplicationEngine>

int main(int argc, char *argv[]) {
    QGuiApplication app(argc, argv);
    QQmlApplicationEngine engine;

    // Load main.qml from the qrc resource registered by
    // qt_add_qml_module() in CMakeLists.txt.
    engine.loadFromModule("VisiCalc", "Main");
    if (engine.rootObjects().isEmpty()) {
        // Fallback to the file path if the module path isn't
        // registered (e.g. if you run the binary outside its
        // build directory).
        engine.load(QUrl(QStringLiteral("qrc:/main.qml")));
    }
    if (engine.rootObjects().isEmpty()) return -1;
    return app.exec();
}

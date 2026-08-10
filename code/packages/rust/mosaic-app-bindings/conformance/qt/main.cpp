#include "MosaicHost.h"

#include <QCoreApplication>
#include <QVariantList>
#include <QVariantMap>

#include <cstdlib>
#include <iostream>
#include <stdexcept>

namespace {
void require(bool condition, const char *assertion)
{
    if (condition) return;
    std::cerr << "Failed assertion: " << assertion << '\n';
    std::exit(EXIT_FAILURE);
}

QVariantMap props(const QVariantMap &update, const char *assertion)
{
    require(!update.contains(QStringLiteral("error")), assertion);
    const auto value = update.value(QStringLiteral("props"));
    require(value.canConvert<QVariantMap>(), assertion);
    return value.toMap();
}

QString expectedPlatform()
{
#if defined(Q_OS_MACOS) || defined(Q_OS_IOS)
    return QStringLiteral("apple");
#elif defined(Q_OS_WIN)
    return QStringLiteral("windows");
#else
    return QStringLiteral("linux");
#endif
}
}

int main(int argc, char *argv[])
{
    QCoreApplication application(argc, argv);
    MosaicHost host;
    const QString mode = argc > 1 ? QString::fromLocal8Bit(argv[1]) : QString();

    if (mode == QStringLiteral("--expect-required-failure")) {
        try {
            host.requireRuntime();
        } catch (const std::exception &exception) {
            const QString message = QString::fromUtf8(exception.what());
            require(message.contains(QStringLiteral("native-complete requires the Mosaic Rust application runtime")),
                    "explicit missing runtime error");
            std::cout << "Mosaic Qt required-runtime failure conformance passed\n";
            return EXIT_SUCCESS;
        }
        require(false, "required runtime should fail when the library is absent");
    }

    host.requireRuntime();
    const QVariantMap slotNames{
        {QStringLiteral("count"), QStringLiteral("count")},
        {QStringLiteral("platform"), QStringLiteral("platform")},
        {QStringLiteral("status"), QStringLiteral("runtimeStatus")},
    };
    const QStringList requiredProps = mode == QStringLiteral("--expect-missing-prop-failure")
        ? QStringList{QStringLiteral("count"), QStringLiteral("missing-prop")}
        : QStringList{QStringLiteral("count"), QStringLiteral("platform"), QStringLiteral("status")};
    host.configureRequiredProps(slotNames, requiredProps);

    if (mode == QStringLiteral("--expect-missing-prop-failure")) {
        try {
            static_cast<void>(host.propsRequired());
        } catch (const std::exception &exception) {
            const QString message = QString::fromUtf8(exception.what());
            require(message.contains(QStringLiteral("missing required MIL prop 'missing-prop'")),
                    "explicit missing required prop error");
            std::cout << "Mosaic Qt missing-prop failure conformance passed\n";
            return EXIT_SUCCESS;
        }
        require(false, "required prop validation should reject a missing prop");
    }

    const auto started = host.props();
    const auto startedProps = host.propsRequired();
    require(started.value(QStringLiteral("revision")).toULongLong() == 1,
            "startup revision");
    require(startedProps.value(QStringLiteral("count")).toLongLong() == 0,
            "initial count");
    require(startedProps.value(QStringLiteral("platform")).toString() == expectedPlatform(),
            "startup platform");
    require(startedProps.value(QStringLiteral("runtimeStatus")).toString() ==
                QStringLiteral("started"),
            "startup mapped status");
    require(!startedProps.contains(QStringLiteral("status")),
            "startup source name removed after mapping");

    const QVariantMap event{
        {QStringLiteral("name"), QStringLiteral("increment")},
        {QStringLiteral("payload"), QVariantMap{{QStringLiteral("amount"), 4}}},
    };
    const auto dispatched = host.handleRequiredEvent(event);
    const auto dispatchedProps = props(dispatched, "dispatch update");
    require(dispatched.value(QStringLiteral("revision")).toULongLong() == 2,
            "dispatch revision");
    require(dispatchedProps.value(QStringLiteral("count")).toLongLong() == 4,
            "dispatched count");
    require(dispatchedProps.value(QStringLiteral("runtimeStatus")).toString() ==
                QStringLiteral("dispatched"),
            "dispatch mapped status");

    const auto snapshotValue = host.snapshot();
    require(snapshotValue.canConvert<QVariantMap>(), "snapshot object");
    const auto snapshot = snapshotValue.toMap();
    require(snapshot.value(QStringLiteral("schema")).toString() ==
                QStringLiteral("mosaic-app-conformance/counter"),
            "snapshot schema");
    require(snapshot.value(QStringLiteral("version")).toUInt() == 1,
            "snapshot version");
    require(snapshot.value(QStringLiteral("bytes")).toList().size() == 8,
            "snapshot bytes");

    const auto restored = host.restore(snapshot);
    const auto restoredProps = props(restored, "restore update");
    require(restored.value(QStringLiteral("revision")).toULongLong() == 3,
            "restore revision");
    require(restoredProps.value(QStringLiteral("count")).toLongLong() == 4,
            "restored count");
    require(restoredProps.value(QStringLiteral("status")).toString() ==
                QStringLiteral("restored"),
            "restore status");

    std::cout << "Mosaic Qt Rust runtime conformance passed\n";
    return EXIT_SUCCESS;
}

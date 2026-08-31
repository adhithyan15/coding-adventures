#include "MosaicHost.h"

#include <QAbstractTableModel>
#include <QApplication>
#include <QCoreApplication>
#include <QHash>
#include <QModelIndex>
#include <QQuickItem>
#include <QQuickStyle>
#include <QQuickView>
#include <QThread>
#include <QUrl>
#include <QVariantList>
#include <QVariantMap>

#include <cstdlib>
#include <iostream>
#include <stdexcept>

class MosaicTableModel final : public QAbstractTableModel
{
    Q_OBJECT
    Q_PROPERTY(QVariantList headers READ headers WRITE setHeaders NOTIFY headersChanged)
    Q_PROPERTY(QVariantList rows READ rows WRITE setRows NOTIFY rowsChanged)

public:
    explicit MosaicTableModel(QObject *parent = nullptr) : QAbstractTableModel(parent) {}
    QVariantList headers() const { return headers_; }
    QVariantList rows() const { return rows_; }
    void setHeaders(const QVariantList &headers) {
        if (headers_ == headers) return;
        beginResetModel(); headers_ = headers; endResetModel(); emit headersChanged();
    }
    void setRows(const QVariantList &rows) {
        if (rows_ == rows) return;
        beginResetModel(); rows_ = rows; endResetModel(); emit rowsChanged();
    }
    int rowCount(const QModelIndex &parent = QModelIndex()) const override {
        return parent.isValid() ? 0 : rows_.size();
    }
    int columnCount(const QModelIndex &parent = QModelIndex()) const override {
        return parent.isValid() ? 0 : headers_.size();
    }
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override {
        if (!index.isValid() || role != Qt::DisplayRole || index.row() >= rows_.size()) return {};
        const auto row = rows_.at(index.row()).toList();
        return index.column() < row.size() ? row.at(index.column()) : QVariant{};
    }
    QHash<int, QByteArray> roleNames() const override {
        return {{Qt::DisplayRole, QByteArrayLiteral("display")}};
    }

signals:
    void headersChanged();
    void rowsChanged();

private:
    QVariantList headers_;
    QVariantList rows_;
};

namespace {
const QString TaskName = QStringLiteral("Native acceptance task");
const QString PersistedTaskName = QStringLiteral("Persisted native task");
const QString Due = QStringLiteral("2026-01-09");
const QString Schedule = QStringLiteral("2026-01-05 → 2026-01-05");

void require(bool condition, const char *assertion)
{
    if (condition) return;
    throw std::runtime_error(std::string("Failed assertion: ") + assertion);
}

QString camelName(const QString &name)
{
    QString result;
    bool uppercase = false;
    for (const auto character : name) {
        if (character == QLatin1Char('-')) { uppercase = true; continue; }
        result += uppercase ? character.toUpper() : character;
        uppercase = false;
    }
    return result;
}

void settle()
{
    for (int i = 0; i < 8; ++i) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 50);
        QThread::msleep(10);
    }
}

QObject *findVisualObject(QObject *root, const QString &name)
{
    if (root->objectName() == name) return root;
    auto *item = qobject_cast<QQuickItem *>(root);
    if (item == nullptr) return nullptr;
    for (auto *child : item->childItems()) {
        if (auto *result = findVisualObject(child, name)) return result;
    }
    return nullptr;
}

QObject *control(QObject *root, const char *name)
{
    auto *result = findVisualObject(root, QString::fromLatin1(name));
    require(result != nullptr, name);
    return result;
}

void click(QObject *root, const char *name)
{
    auto *button = control(root, name);
    require(QMetaObject::invokeMethod(button, "clicked", Qt::DirectConnection), name);
    settle();
}

bool hasText(QObject *root, const QString &expected)
{
    if (root->property("text").toString() == expected) return true;
    auto *item = qobject_cast<QQuickItem *>(root);
    if (item == nullptr) return false;
    for (auto *child : item->childItems()) {
        if (child->property("text").toString() == expected) return true;
        if (hasText(child, expected)) return true;
    }
    return false;
}

QVariantList taskRows(QObject *root)
{
    return root->property("taskRows").toList();
}

void requireTask(QObject *root, const QString &name)
{
    const auto rows = taskRows(root);
    require(rows.size() == 1, "one task row");
    const auto row = rows.first().toList();
    require(row.size() >= 4, "task row projection width");
    require(row.at(1).toString() == name, "task name projection");
    require(row.at(2).toString() == QStringLiteral("due ") + Due, "task due projection");
    require(row.at(3).toString() == Schedule, "Rust schedule start/finish projection");
    require(hasText(root, name), "task name rendered by emitted control");
    require(hasText(root, QStringLiteral("due ") + Due), "due date rendered by emitted control");
    require(hasText(root, Schedule), "schedule rendered by emitted control");
}
}

int main(int argc, char *argv[])
{
    if (qEnvironmentVariableIsEmpty("QT_QUICK_CONTROLS_STYLE")) {
        QQuickStyle::setStyle(QStringLiteral("Basic"));
    }
    QApplication app(argc, argv);
    try {
        MosaicHost mosaicHost;
        mosaicHost.requireRuntime();
        const auto rawUpdate = mosaicHost.props();
        const auto rawProps = rawUpdate.value(QStringLiteral("props")).toMap();
        QVariantMap slotNames;
        QStringList requiredProps;
        for (auto iterator = rawProps.cbegin(); iterator != rawProps.cend(); ++iterator) {
            slotNames.insert(iterator.key(), camelName(iterator.key()));
            requiredProps.append(iterator.key());
        }
        mosaicHost.configureRequiredProps(slotNames, requiredProps);

        QQuickView view;
        view.setResizeMode(QQuickView::SizeRootObjectToView);
        view.resize(1100, 800);
        QVariantList nativeTableModels{
            QVariant::fromValue(static_cast<QObject *>(new MosaicTableModel(&view))),
        };
        auto initialProperties = mosaicHost.propsRequired();
        initialProperties.insert(QStringLiteral("mosaicHost"),
                                 QVariant::fromValue(static_cast<QObject *>(&mosaicHost)));
        initialProperties.insert(QStringLiteral("mosaicNativeTableModels"), nativeTableModels);
        view.setInitialProperties(initialProperties);
        view.setSource(QUrl(QStringLiteral("qrc:/qt/qml/Mosaic/TaskApp/TaskApp.qml")));
        require(view.status() == QQuickView::Ready && view.rootObject() != nullptr,
                "generated QML root instantiated");
        auto *root = view.rootObject();
        view.show();
        settle();

        const bool restoredOnLaunch = qEnvironmentVariable("MOSAIC_EXPECT_RESTORED") ==
            QStringLiteral("1");
        if (restoredOnLaunch) {
            requireTask(root, PersistedTaskName);
            click(root, "del-btn");
            require(taskRows(root).isEmpty(), "delete restored task through emitted control");
            std::cout << "TaskApp Qt persisted UI restart conformance passed\n";
            return EXIT_SUCCESS;
        }

        require(taskRows(root).isEmpty(), "fresh task list");
        const auto before = mosaicHost.snapshot();
        const auto rejected = mosaicHost.handleEvent({
            {QStringLiteral("name"), QStringLiteral("onNewTaskNameChange")},
            {QStringLiteral("payload"), QVariantMap{{QStringLiteral("value"), 7}}},
        });
        require(rejected.contains(QStringLiteral("error")), "invalid input rejected");
        require(mosaicHost.snapshot() == before, "invalid input preserved state");

        control(root, "name-input")->setProperty("text", TaskName);
        settle();
        control(root, "due-input")->setProperty("text", Due);
        settle();
        click(root, "add-btn");
        require(taskRows(root).first().toList().at(3).toString().isEmpty(),
                "Board mode hides schedule");
        click(root, "complexity-toggle");
        requireTask(root, TaskName);

        click(root, "toggle");
        require(taskRows(root).first().toList().at(0).toString() == QStringLiteral("✓"),
                "complete task through emitted control");
        require(hasText(root, QStringLiteral("100%")), "completion rendered");
        click(root, "toggle");
        require(taskRows(root).first().toList().at(0).toString() == QStringLiteral("○"),
                "reopen task through emitted control");
        click(root, "del-btn");
        require(taskRows(root).isEmpty(), "delete task through emitted control");

        control(root, "name-input")->setProperty("text", PersistedTaskName);
        settle();
        control(root, "due-input")->setProperty("text", Due);
        settle();
        click(root, "add-btn");
        requireTask(root, PersistedTaskName);
        std::cout << "TaskApp Qt emitted-control lifecycle conformance passed\n";
        return EXIT_SUCCESS;
    } catch (const std::exception &exception) {
        std::cerr << exception.what() << '\n';
        return EXIT_FAILURE;
    }
}

#include "main.moc"

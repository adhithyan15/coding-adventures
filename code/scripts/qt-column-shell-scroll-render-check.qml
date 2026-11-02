// Does a vertical HostScroll in a plain Column shell actually scroll on Qt?
// (UI21 §2.2, the EN-6 follow-up to #18477)
//
// The generated QML for this shape compiled, linted and passed every gate
// while it could not scroll. The Column root was a content-sized layout the
// window never reached, and `flex-grow` had no lowering. The ScrollView was
// therefore exactly as tall as its content: a viewport with nothing to
// scroll. No text search sees that. A measurement does.
//
// The same check runs on a second fixture,
// code/packages/rust/mosaic-emit-qt/fixtures/content-sized-regions (UI21
// §2.2, rules 6 and 7). Its scrolled content is a padded styled Box with no
// size of its own and a two-child If/Else body, each holding a 400px block.
// Both used to publish 0 x 0, and either one alone leaves the content
// shorter than the viewport, so the contentHeight assertion below fails if
// either regresses. Both fixtures export `Shell` with a `title` slot.
//
// Run from the generated Qt output of either fixture, the directory where
// `Shell.qml` and its `qmldir` live:
//
//     QT_QPA_PLATFORM=offscreen qml qt-column-shell-scroll-render-check.qml
//
// Exits 0 when every assertion holds, 1 otherwise, naming what it measured.
import QtQuick
import QtQuick.Controls

Window {
    id: win
    visible: true
    width: 900
    height: 600

    Shell {
        id: shell
        // The generated shell hosts the root through a QQuickView with
        // SizeRootObjectToView. That sizes the root to the window, and so
        // does this anchor.
        anchors.fill: parent

        // `--emit-project --profile native-complete` generates
        // `required property var mosaicHost`. This check measures geometry
        // and never dispatches an event, so a stub is enough (the same as
        // qt-navigation-split-render-check.qml).
        mosaicHost: QtObject {
            function handleEvent(event) {
                return null
            }
            function handleRequiredEvent(event) {
                return null
            }
        }

        title: "Header"
    }

    // Duck-typed on purpose: `availableHeight` belongs to a Control, and
    // `contentHeight` to a ScrollView's Flickable surface. The generated
    // ScrollView carries no objectName to find it by.
    function findScrollView(node) {
        if (node.availableHeight !== undefined && node.contentHeight !== undefined
                && node.contentItem !== undefined && node.contentItem.contentY !== undefined)
            return node
        for (var i = 0; i < node.children.length; i++) {
            var hit = findScrollView(node.children[i])
            if (hit)
                return hit
        }
        return null
    }

    property int failures: 0

    function check(label, actual, ok) {
        if (ok) {
            console.log("  ok   " + label + " = " + actual)
        } else {
            console.log("  FAIL " + label + " = " + actual)
            win.failures += 1
        }
    }

    // Layouts settle on a polish pass after the first frame, so the read
    // waits a moment.
    Timer {
        id: measure
        interval: 500
        running: true
        onTriggered: {
            console.log("column shell scroll render check, window = " + win.width + "x" + win.height)

            win.check("root Shell height", Math.round(shell.height), Math.round(shell.height) === win.height)

            var scroll = win.findScrollView(shell)
            if (!scroll) {
                console.log("  FAIL no ScrollView found under the shell")
                Qt.exit(1)
                return
            }

            // Bounded: the viewport is the window less the shell's padding
            // and the header, which proves the shell reached the window AND
            // the scroll took the remainder.
            win.check("ScrollView height", Math.round(scroll.height),
                      scroll.height > 200 && scroll.height < win.height)
            // Rule 4: a vertical viewport fills its column's width. If it
            // did not, it would be laid out zero pixels wide.
            win.check("ScrollView width", Math.round(scroll.width),
                      scroll.width > win.width / 2 && scroll.width <= win.width)
            // Something to scroll: the content (a 1600px block, or two
            // 400px blocks in the content-sized-regions fixture) is taller
            // than any viewport this window can give it.
            win.check("ScrollView contentHeight", Math.round(scroll.contentHeight),
                      scroll.contentHeight > scroll.height)

            // It scrolls. `visibleArea.heightRatio` is the Flickable's own
            // account of the viewport: the fraction of the content it shows,
            // under 1 only when there is more to scroll to. (Setting
            // `contentY` proves nothing: a Flickable keeps a programmatic
            // offset even when its content fits, so that check passed on the
            // broken output too.)
            var area = scroll.contentItem.visibleArea
            win.check("visible fraction of the content", area.heightRatio.toFixed(3),
                      area.heightRatio > 0 && area.heightRatio < 1)
            if (win.failures > 0) {
                console.log("column shell scroll render check FAILED: " + win.failures + " assertion(s)")
                Qt.exit(1)
            } else {
                console.log("column shell scroll render check passed")
                Qt.exit(0)
            }
        }
    }
}

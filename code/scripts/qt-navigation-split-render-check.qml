// Does the Qt navigation split actually occupy the window? (#15833)
//
// The Qt lane used to check this by grepping the generated QML for
// `SplitView {`. That passed for months against markup that laid out to
// 0 x 0 and drew nothing at any window size: `Layout.fillWidth` is inert
// outside a `RowLayout`/`ColumnLayout`, QML warns about neither, and
// `SplitView` -- unlike `RowLayout` -- derives no implicit size from its
// panes. A text search cannot see any of that. A measurement can.
//
// Run from the generated Qt output directory, which is where `Shell.qml`
// and its `qmldir` live:
//
//     QT_QPA_PLATFORM=offscreen qml qt-navigation-split-render-check.qml
//
// Exits 0 when every assertion holds, 1 otherwise, naming what it measured.
import QtQuick
import QtQuick.Controls

Window {
    id: win
    visible: true
    width: 1200
    height: 600

    Shell {
        id: shell
        anchors.fill: parent
        workspaceName: "Workspace"
        inboxLabel: "Inbox"
    }

    function findByName(node, name) {
        if (node.objectName === name)
            return node
        for (var i = 0; i < node.children.length; i++) {
            var hit = findByName(node.children[i], name)
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

    // One frame is not enough: SplitView distributes its panes on a later
    // polish pass, so widths read at t=0 are still zero even when correct.
    Timer {
        interval: 500
        running: true
        onTriggered: {
            console.log("navigation split render check, window = " + win.width + "x" + win.height)

            win.check("root Shell width", Math.round(shell.width), Math.round(shell.width) === win.width)

            var outer = win.findByName(shell, "workbench")
            if (!outer) {
                console.log("  FAIL outer split 'workbench' not found")
                win.failures += 1
            } else {
                // The root split must fill the component, not sit at its
                // implicit size -- which is what 0 x 0 was.
                win.check("outer SplitView width", Math.round(outer.width),
                          Math.round(outer.width) === win.width)
                win.check("outer SplitView height", Math.round(outer.height),
                          Math.round(outer.height) === win.height)
            }

            var inner = win.findByName(shell, "app-shell")
            if (!inner) {
                console.log("  FAIL inner split 'app-shell' not found")
                win.failures += 1
            } else {
                win.check("inner SplitView width", Math.round(inner.width), inner.width > 0)
                // `contentChildren` is the panes; `children` also holds the
                // internal drag handles.
                var panes = inner.contentChildren
                if (panes.length !== 2) {
                    console.log("  FAIL inner split pane count = " + panes.length)
                    win.failures += 1
                } else {
                    // `pane-width: 236` in the fixture, honoured exactly.
                    win.check("pane width", Math.round(panes[0].width),
                              Math.round(panes[0].width) === 236)
                    // The detail is what the user reads. It had better be
                    // most of the window.
                    win.check("detail width", Math.round(panes[1].width),
                              panes[1].width > 400)
                    win.check("detail height", Math.round(panes[1].height),
                              panes[1].height > 0)
                }
            }

            if (win.failures > 0)
                console.log("navigation split render check FAILED (" + win.failures + ")")
            else
                console.log("navigation split render check passed")
            Qt.exit(win.failures > 0 ? 1 : 0)
        }
    }
}

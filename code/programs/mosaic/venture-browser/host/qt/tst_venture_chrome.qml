import QtQuick 2.15
import QtTest 1.3
import ".."

TestCase {
    name: "VentureChromeInteraction"
    width: 1100
    height: 800
    when: windowShown

    QtObject {
        id: recordingHost

        property var events: []

        function reset() {
            events = []
        }

        function handleEvent(event) {
            events = events.concat([event])
            if (event.event === "onNavigate") {
                return {
                    "props": {
                        "statusText": "Navigated through MosaicHost"
                    }
                }
            }
            if (event.event === "onFindOpen") {
                return { "props": { "findOpen": true } }
            }
            if (event.event === "onFindClose") {
                return { "props": { "findOpen": false, "findQuery": "", "findResultLabel": "" } }
            }
            return null
        }
    }

    VentureChrome {
        id: chrome
        anchors.fill: parent
        mosaicHost: recordingHost
    }

    function nativeControl(objectName) {
        const control = findChild(chrome, objectName)
        verify(control !== null, "missing native control " + objectName)
        return control
    }

    function hydrate(disabled) {
        chrome.applyMosaicResponse({
            "props": {
                "address": "http://venture.test/start",
                "pageTitle": "Venture Qt acceptance",
                "statusText": "Ready",
                "backDisabled": disabled,
                "forwardDisabled": disabled,
                "bookmarkLabel": "Bookmark",
                "bookmarkDisabled": disabled,
                "copyAddressDisabled": disabled,
                "openPageDisabled": disabled,
                "savePageDisabled": disabled,
                "viewSourceDisabled": disabled,
                "findOpen": false,
                "findQuery": "",
                "findResultLabel": "",
                "findDisabled": disabled,
                "navigationDisabled": disabled
            }
        })
        wait(0)
    }

    function init() {
        hydrate(true)
        recordingHost.reset()
    }

    function test_host_hydration_reaches_native_controls() {
        compare(chrome.address, "http://venture.test/start")
        compare(chrome.pageTitle, "Venture Qt acceptance")
        compare(chrome.statusText, "Ready")
        verify(!nativeControl("back-button").enabled)
        verify(!nativeControl("forward-button").enabled)
        verify(!nativeControl("reload-button").enabled)
        verify(!nativeControl("copy-address-button").enabled)
        verify(!nativeControl("open-page-button").enabled)
        verify(!nativeControl("save-page-button").enabled)
        verify(!nativeControl("print-page-button").enabled)
        verify(!nativeControl("share-page-button").enabled)
        verify(!nativeControl("view-source-button").enabled)
        verify(!nativeControl("find-button").enabled)
        verify(nativeControl("address-input").readOnly)
        verify(!nativeControl("go-button").enabled)
        verify(nativeControl("mosaic-host-surface") !== null)
    }

    function test_disabled_native_controls_suppress_dispatch() {
        mouseClick(nativeControl("back-button"))
        mouseClick(nativeControl("forward-button"))
        mouseClick(nativeControl("reload-button"))
        mouseClick(nativeControl("copy-address-button"))
        mouseClick(nativeControl("open-page-button"))
        mouseClick(nativeControl("save-page-button"))
        mouseClick(nativeControl("print-page-button"))
        mouseClick(nativeControl("share-page-button"))
        mouseClick(nativeControl("view-source-button"))
        mouseClick(nativeControl("find-button"))
        mouseClick(nativeControl("go-button"))
        compare(recordingHost.events.length, 0)
    }

    function test_address_return_crosses_the_mosaic_host_seam() {
        hydrate(false)
        recordingHost.reset()

        const input = nativeControl("address-input")
        verify(!input.readOnly)
        input.forceActiveFocus()
        input.text = "http://venture.test/next"
        wait(0)
        compare(recordingHost.events.length, 1)
        compare(recordingHost.events[0].event, "onAddressChange")
        compare(recordingHost.events[0].value, "http://venture.test/next")

        keyClick(Qt.Key_Return)
        wait(0)
        compare(recordingHost.events.length, 2)
        compare(recordingHost.events[1].event, "onNavigate")
        compare(chrome.statusText, "Navigated through MosaicHost")
    }

    function test_go_crosses_the_mosaic_host_seam() {
        hydrate(false)
        recordingHost.reset()
        const goButton = nativeControl("go-button")
        verify(goButton.enabled)
        goButton.forceActiveFocus()
        keyClick(Qt.Key_Space)
        wait(0)
        compare(recordingHost.events.length, 1)
        compare(recordingHost.events[0].event, "onNavigate")
        compare(chrome.statusText, "Navigated through MosaicHost")
    }

    function test_view_source_crosses_the_mosaic_host_seam() {
        hydrate(false)
        recordingHost.reset()
        const viewSourceButton = nativeControl("view-source-button")
        verify(viewSourceButton.enabled)
        viewSourceButton.forceActiveFocus()
        keyClick(Qt.Key_Space)
        wait(0)
        compare(recordingHost.events.length, 1)
        compare(recordingHost.events[0].event, "onViewSource")
    }

    function test_copy_address_crosses_the_mosaic_host_seam() {
        hydrate(false)
        recordingHost.reset()
        const copyAddressButton = nativeControl("copy-address-button")
        verify(copyAddressButton.enabled)
        copyAddressButton.forceActiveFocus()
        keyClick(Qt.Key_Space)
        wait(0)
        compare(recordingHost.events.length, 1)
        compare(recordingHost.events[0].event, "onCopyAddress")
    }

    function test_open_page_crosses_the_mosaic_host_seam() {
        hydrate(false)
        recordingHost.reset()
        const openPageButton = nativeControl("open-page-button")
        verify(openPageButton.enabled)
        openPageButton.forceActiveFocus()
        keyClick(Qt.Key_Space)
        wait(0)
        compare(recordingHost.events.length, 1)
        compare(recordingHost.events[0].event, "onOpenPageInNewWindow")
    }

    function test_save_page_crosses_the_mosaic_host_seam() {
        hydrate(false)
        recordingHost.reset()
        const savePageButton = nativeControl("save-page-button")
        verify(savePageButton.enabled)
        savePageButton.forceActiveFocus()
        keyClick(Qt.Key_Space)
        wait(0)
        compare(recordingHost.events.length, 1)
        compare(recordingHost.events[0].event, "onSavePage")
    }

    function test_print_page_crosses_the_mosaic_host_seam() {
        hydrate(false)
        recordingHost.reset()
        const printPageButton = nativeControl("print-page-button")
        verify(printPageButton.enabled)
        printPageButton.forceActiveFocus()
        keyClick(Qt.Key_Space)
        wait(0)
        compare(recordingHost.events.length, 1)
        compare(recordingHost.events[0].event, "onPrintPage")
    }

    function test_share_page_crosses_the_mosaic_host_seam() {
        hydrate(false)
        recordingHost.reset()
        const sharePageButton = nativeControl("share-page-button")
        verify(sharePageButton.enabled)
        sharePageButton.forceActiveFocus()
        keyClick(Qt.Key_Space)
        wait(0)
        compare(recordingHost.events.length, 1)
        compare(recordingHost.events[0].event, "onSharePage")
    }

    function test_find_actions_cross_the_mosaic_host_seam() {
        hydrate(false)
        recordingHost.reset()
        const openButton = nativeControl("find-button")
        openButton.forceActiveFocus()
        keyClick(Qt.Key_Space)
        wait(0)
        compare(recordingHost.events[0].event, "onFindOpen")
        const input = nativeControl("find-input")
        input.forceActiveFocus()
        input.text = "venture"
        wait(0)
        compare(recordingHost.events[1].event, "onFindChange")
        compare(recordingHost.events[1].value, "venture")
        const nextButton = nativeControl("find-next-button")
        nextButton.forceActiveFocus()
        keyClick(Qt.Key_Space)
        wait(0)
        compare(recordingHost.events[2].event, "onFindNext")

        const previousButton = nativeControl("find-previous-button")
        previousButton.forceActiveFocus()
        keyClick(Qt.Key_Space)
        wait(0)
        compare(recordingHost.events[3].event, "onFindPrevious")

        const closeButton = nativeControl("find-close-button")
        closeButton.forceActiveFocus()
        keyClick(Qt.Key_Space)
        wait(0)
        compare(recordingHost.events[4].event, "onFindClose")
        verify(findChild(chrome, "find-input") === null)
    }
}

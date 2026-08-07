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
        verify(nativeControl("address-input").readOnly)
        verify(!nativeControl("go-button").enabled)
        verify(nativeControl("mosaic-host-surface") !== null)
    }

    function test_disabled_native_controls_suppress_dispatch() {
        mouseClick(nativeControl("back-button"))
        mouseClick(nativeControl("forward-button"))
        mouseClick(nativeControl("reload-button"))
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
}

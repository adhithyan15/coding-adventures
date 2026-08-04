import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.click
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performImeAction
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTextReplacement
import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress
import java.nio.charset.StandardCharsets
import java.util.concurrent.Executors
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import org.junit.Rule
import org.junit.Test

private class VenturePageServer : AutoCloseable {
    private val executor = Executors.newCachedThreadPool()
    private val server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)

    val origin: String
        get() = "http://127.0.0.1:${server.address.port}"

    init {
        server.executor = executor
        server.createContext("/") { exchange ->
            val html = when (exchange.requestURI.path) {
                "/start" -> """
                    <html><head><title>Compose Start</title></head><body>
                    <a href="/link">Open the Compose link target</a>
                    ${List(80) { index -> "<p>scroll row $index</p>" }.joinToString("")}
                    </body></html>
                """.trimIndent()
                "/target" -> """
                    <html><head><title>Compose Address Target</title></head>
                    <body>address navigation reached the shared browser</body></html>
                """.trimIndent()
                "/link" -> """
                    <html><head><title>Compose Link Target</title></head>
                    <body>native Compose pointer activation reached Cairo</body></html>
                """.trimIndent()
                else -> "<html><head><title>Missing</title></head><body>missing page</body></html>"
            }.toByteArray(StandardCharsets.UTF_8)
            exchange.responseHeaders.add("Content-Type", "text/html; charset=utf-8")
            exchange.sendResponseHeaders(if (exchange.requestURI.path in setOf("/start", "/target", "/link")) 200 else 404, html.size.toLong())
            exchange.responseBody.use { it.write(html) }
        }
        server.start()
    }

    override fun close() {
        server.stop(0)
        executor.shutdownNow()
    }
}

class VentureChromeInteractionTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun packageOwnedComposeShellDrivesTheLiveSharedBrowserAndCairoPage() {
        val libraryPath = assertNotNull(
            System.getenv("VENTURE_BROWSER_COMPOSE_LIBRARY"),
            "the direct Compose gate requires the shared Rust bridge",
        )
        val server = VenturePageServer()
        val host = MosaicHost.open(libraryPath, "${server.origin}/start")
        try {
            println("compose-live-stage=host-open")
            rule.setContent { MosaicApp(host) }
            rule.waitUntil(10_000) { host.renderedFrameCount.get() > 0 }
            rule.onNodeWithText("Compose Start").assertExists()
            val surface = rule.onNodeWithTag("venture-content-surface").assertExists()
            println("compose-live-stage=shell-mounted")

            rule.onNodeWithTag("back-button").assertIsNotEnabled()
            rule.onNodeWithTag("forward-button").assertIsNotEnabled()

            val targetUrl = "${server.origin}/target"
            val address = rule.onNodeWithTag("address-input").assertIsEnabled()
            address.performTextReplacement(targetUrl)
            address.performImeAction()
            rule.waitUntil(10_000) {
                runCatching { rule.onNodeWithText("Compose Address Target").assertExists() }.isSuccess
            }
            println("compose-live-stage=address-navigation")

            rule.onNodeWithTag("back-button").assertIsEnabled().performClick()
            rule.waitUntil(10_000) {
                runCatching { rule.onNodeWithText("Compose Start").assertExists() }.isSuccess
            }
            rule.onNodeWithTag("forward-button").assertIsEnabled().performClick()
            rule.waitUntil(10_000) {
                runCatching { rule.onNodeWithText("Compose Address Target").assertExists() }.isSuccess
            }
            rule.onNodeWithTag("back-button").performClick()
            rule.waitUntil(10_000) {
                runCatching { rule.onNodeWithText("Compose Start").assertExists() }.isSuccess
            }
            println("compose-live-stage=history")

            val beforeScroll = assertNotNull(host.scrollMetrics)
            assertTrue(beforeScroll.getValue("max") > 0.0)
            surface.performMouseInput {
                moveTo(Offset(500f, 300f))
                scroll(320f)
            }
            rule.waitUntil(10_000) { (host.scrollMetrics?.get("offset") ?: 0.0) > 0.0 }
            surface.performMouseInput { scroll(-10_000f) }
            rule.waitUntil(10_000) { host.scrollMetrics?.get("offset") == 0.0 }
            println("compose-live-stage=scroll")

            val linkUrl = "${server.origin}/link"
            surface.performMouseInput { moveTo(Offset(32f, 26f)) }
            rule.waitUntil(10_000) { host.statusText == linkUrl }
            rule.onNodeWithText(linkUrl).assertExists()
            println("compose-live-stage=hover")

            surface.performMouseInput { click(Offset(32f, 26f)) }
            rule.waitUntil(10_000) {
                runCatching { rule.onNodeWithText("Compose Link Target").assertExists() }.isSuccess
            }
            assertEquals(linkUrl, host.props()["props"].let { props ->
                (props as Map<*, *>)["address"]
            })
            assertTrue(host.renderedFrameCount.get() >= 2)
            println("compose-live-stage=link")
        } finally {
            host.close()
            server.close()
        }
    }
}

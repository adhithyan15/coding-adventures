package org.mosaic.flux

import kotlin.test.Test
import kotlin.test.assertEquals

private data class DtS(val v: Int = 0)

private class DtBump : MosaicAction<DtS> {
    override fun apply(state: DtS): DtS = state.copy(v = state.v + 1)
}

class DevToolsTest {
    @Test
    fun callable() {
        val m = devToolsMiddleware<DtS>()
        m(DtBump(), DtS(), DtS(v = 1))
    }

    @Test
    fun customStoreName() {
        val m = devToolsMiddleware<DtS>("my-grid")
        m(DtBump(), DtS(), DtS(v = 1))
    }

    @Test
    fun integratesWithStore() {
        var probeRuns = 0
        val store = MosaicStore<DtS>(
            DtS(),
            listOf(devToolsMiddleware(), { _, _, _ -> probeRuns++ }),
        )
        store.dispatch(DtBump())
        store.dispatch(DtBump())
        assertEquals(2, probeRuns)
        assertEquals(2, store.state.v)
    }
}

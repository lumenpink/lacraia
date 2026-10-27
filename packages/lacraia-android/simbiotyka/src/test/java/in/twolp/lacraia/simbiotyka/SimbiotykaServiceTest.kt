package `in`.twolp.lacraia.simbiotyka

import org.junit.Assert.assertEquals
import org.junit.Test

class SimbiotykaServiceTest {

    @Test
    fun testActionStopServiceConstant() {
        assertEquals(
            "in.twolp.lacraia.simbiotyka.ACTION_STOP_SERVICE",
            SimbiotykaService.ACTION_STOP_SERVICE
        )
    }

    @Test
    fun testActionNamespaceIntegrity() {
        assert(SimbiotykaService.ACTION_STOP_SERVICE.startsWith("in.twolp.lacraia.simbiotyka."))
    }
}

package com.learningbymachine.kkft.libraries.shared.math

import kotlin.test.Test
import kotlin.test.assertEquals

class AndroidFibiTest {

    @Test
    fun testThirdElement() {
        assertEquals(3, generateFibi().take(3).last())
    }
}
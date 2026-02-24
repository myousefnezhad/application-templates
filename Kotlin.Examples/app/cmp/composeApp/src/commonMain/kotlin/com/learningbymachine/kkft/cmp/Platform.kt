package com.learningbymachine.kkft.cmp

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform
package com.learningbymachine.kkft.web.pages

import androidx.compose.runtime.Composable
import com.learningbymachine.kkft.web.components.layouts.PageLayoutData
import com.varabyte.kobweb.compose.foundation.layout.Box
import com.varabyte.kobweb.compose.foundation.layout.Column
import com.varabyte.kobweb.compose.foundation.layout.Row
import com.varabyte.kobweb.compose.ui.Modifier
import com.varabyte.kobweb.compose.ui.modifiers.gap
import com.varabyte.kobweb.core.Page
import com.varabyte.kobweb.core.data.add
import com.varabyte.kobweb.core.init.InitRoute
import com.varabyte.kobweb.core.init.InitRouteContext
import com.varabyte.kobweb.core.layout.Layout
import com.varabyte.kobweb.silk.components.text.SpanText
import com.varabyte.kobweb.silk.style.toModifier
import org.jetbrains.compose.web.css.cssRem

@InitRoute
fun initLoginPage(ctx: InitRouteContext) {
    ctx.data.add(PageLayoutData("Login"))
}

@Page
@Layout(".components.layouts.PageLayout")
@Composable
fun LoginPage() {
    Row(HeroContainerStyle.toModifier()) {
        Box {
            Column(Modifier.gap(2.cssRem)) {
                SpanText("Login")
            }
        }
    }
}
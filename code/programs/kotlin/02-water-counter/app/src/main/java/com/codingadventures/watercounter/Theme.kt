package com.codingadventures.watercounter

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext

@Composable
fun WaterCounterTheme(content: @Composable () -> Unit) {
    val colorScheme = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.S) {
        dynamicLightColorScheme(LocalContext.current)
    } else {
        lightColorScheme()
    }
    MaterialTheme(colorScheme = colorScheme, content = content)
}

// Theme.kt
// Material Design 3 theme for the app.
//
// Material Design is Google's design system — the Android equivalent
// of Apple's Human Interface Guidelines. Material 3 (2022) uses
// dynamic colour, which on Android 12+ adapts to the user's wallpaper.

package com.codingadventures.helloworld

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext

@Composable
fun HelloWorldTheme(content: @Composable () -> Unit) {
    // Dynamic colour is available on Android 12+ (API 31).
    // On older versions we fall back to a static blue colour scheme.
    val colorScheme = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.S) {
        dynamicLightColorScheme(LocalContext.current)
    } else {
        lightColorScheme()
    }

    MaterialTheme(
        colorScheme = colorScheme,
        content = content
    )
}

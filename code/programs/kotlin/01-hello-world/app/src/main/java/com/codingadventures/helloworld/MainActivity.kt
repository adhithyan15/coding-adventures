// MainActivity.kt
// The entry point of the Android app.
//
// On Android, an Activity is a single screen. It's the equivalent of
// a ViewController in UIKit or a View in SwiftUI. Every Android app
// has at least one Activity — this is ours.
//
// We're using Jetpack Compose for the UI, which is Android's modern
// declarative UI framework introduced in 2021. If you've used SwiftUI,
// Compose will feel immediately familiar — both describe *what* the UI
// should look like rather than *how* to build it step by step.

package com.codingadventures.helloworld

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.WaterDrop
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // enableEdgeToEdge lets the app draw behind the system bars
        // (status bar, navigation bar) for a modern full-screen look.
        enableEdgeToEdge()

        // setContent is the Compose equivalent of SwiftUI's body.
        // It hands the screen over to a Composable function.
        setContent {
            HelloWorldTheme {
                // Scaffold provides the basic Material Design layout
                // structure — equivalent to a NavigationView in SwiftUI.
                Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                    HelloWorldScreen(modifier = Modifier.padding(innerPadding))
                }
            }
        }
    }
}

// @Composable marks a function as a UI building block.
// This is Android's equivalent of `var body: some View` in SwiftUI.
//
// SwiftUI:           Compose:
// struct MyView: View   @Composable fun MyScreen()
// var body: some View   (the function body IS the UI)
// VStack { }            Column { }
// HStack { }            Row { }
// Text("hello")         Text("hello")

@Composable
fun HelloWorldScreen(modifier: Modifier = Modifier) {
    // Column is the vertical equivalent of SwiftUI's VStack.
    Column(
        modifier = modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        // Material Icons includes a water drop — same concept as SF Symbols on iOS.
        Icon(
            imageVector = Icons.Filled.WaterDrop,
            contentDescription = "Water drop",
            tint = MaterialTheme.colorScheme.primary,
            modifier = Modifier.padding(bottom = 16.dp).let {
                // Scale the icon up — default Icon size is 24dp, we want 64dp
                it
            }
        )

        Text(
            text = "Hello, World!",
            fontSize = 28.sp,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.padding(bottom = 8.dp)
        )

        Text(
            text = "Welcome to your first Android app.",
            fontSize = 16.sp,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

// @Preview renders the composable in Android Studio's preview panel
// without running the emulator — same as #Preview in SwiftUI.
@Preview(showBackground = true)
@Composable
fun HelloWorldScreenPreview() {
    HelloWorldTheme {
        HelloWorldScreen()
    }
}

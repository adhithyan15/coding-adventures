package com.codingadventures.waterpersist

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

class MainActivity : ComponentActivity() {
    private val viewModel: WaterViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            MaterialTheme {
                WaterScreen(viewModel)
            }
        }
    }
}

@Composable
fun WaterScreen(viewModel: WaterViewModel) {
    val totalMl by viewModel.todayTotalMl.collectAsState()
    val goalMl = 2000
    val progress by animateFloatAsState(
        targetValue = (totalMl.toFloat() / goalMl).coerceIn(0f, 1f),
        label = "progress"
    )
    val goalMet = totalMl >= goalMl

    Scaffold { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            Text("💧", fontSize = 64.sp)
            Spacer(Modifier.height(16.dp))
            Text(
                "$totalMl ml",
                fontSize = 48.sp,
                fontWeight = FontWeight.Bold,
                color = if (goalMet) Color(0xFF4CAF50) else MaterialTheme.colorScheme.onSurface
            )
            Text("today", style = MaterialTheme.typography.bodyMedium, color = Color.Gray)
            Spacer(Modifier.height(16.dp))
            LinearProgressIndicator(
                progress = { progress },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(8.dp),
                color = if (goalMet) Color(0xFF4CAF50) else MaterialTheme.colorScheme.primary
            )
            Text(
                "$totalMl of $goalMl ml",
                style = MaterialTheme.typography.bodySmall,
                color = Color.Gray
            )
            Spacer(Modifier.height(32.dp))
            Button(
                onClick = { viewModel.logDrink() },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(56.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = if (goalMet) Color(0xFF4CAF50) else MaterialTheme.colorScheme.primary
                )
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(if (goalMet) "Log Another" else "Log a Drink", fontWeight = FontWeight.Bold)
                    Text("+250 ml", style = MaterialTheme.typography.bodySmall)
                }
            }
            Spacer(Modifier.height(16.dp))
            Text("💾 Saved locally", style = MaterialTheme.typography.labelSmall, color = Color.Gray)
        }
    }
}

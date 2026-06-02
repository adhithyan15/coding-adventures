package com.example.visicalc

// FormulaBar.kt — hand-written placeholder for the FormulaBar
// composable that mosaic-emit-compose will eventually generate.
//
// Visual contract per UI26 §2.2 + Grid.dark.msl:
//
//   ┌──────┬─────────────────────────────────────┐
//   │  A1  │  =SUM(B1:B5)                         │
//   └──────┴─────────────────────────────────────┘
//   ←cell→ ←formula text field, focusable, edits→
//
// Wire shape mirrors the other backends:
//
//   FormulaBar(cellAddress, formula, onFormulaChange, onCommit, onCancel)
//
// The eventual generated composable should accept exactly these
// parameters so the host (Main.kt) can swap to the generated version
// without touching its call site.

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable
fun FormulaBar(
    cellAddress: String,
    formula: String,
    onFormulaChange: (String) -> Unit,
    onCommit: () -> Unit,
    onCancel: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(Color(0xFF252526)),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // Cell-address chip on the left.  Fixed 48dp width so the
        // formula field aligns with the row-label column of the
        // grid below.
        Box(
            modifier = Modifier
                .width(48.dp)
                .padding(8.dp),
        ) {
            Text(
                text = cellAddress,
                color = Color(0xFFCCCCCC),
                fontSize = 12.sp,
                fontFamily = FontFamily.Monospace,
            )
        }

        // Formula text field.  BasicTextField is the Compose primitive
        // — no platform-decoration like background or border, which
        // matches the dark-theme contract.  We add our own inner
        // padding and a subtle background tint.
        BasicTextField(
            value = formula,
            onValueChange = onFormulaChange,
            modifier = Modifier
                .fillMaxWidth()
                .background(Color(0xFF1E1E1E))
                .padding(8.dp),
            textStyle = TextStyle(
                color = Color(0xFFCCCCCC),
                fontSize = 12.sp,
                fontFamily = FontFamily.Monospace,
            ),
            singleLine = true,
            // Commit + cancel handlers are wired below via a
            // KeyEvent listener on the underlying field; for v0.1.0
            // we keep the wiring at the Modifier level out of scope
            // (Compose Desktop's onPreviewKeyEvent ergonomics deserve
            // their own pass when the emitter lands).
        )

        // Suppress unused-parameter warnings until the key handlers
        // are wired in v0.2.0.
        @Suppress("UNUSED_EXPRESSION") onCommit
        @Suppress("UNUSED_EXPRESSION") onCancel
    }
}

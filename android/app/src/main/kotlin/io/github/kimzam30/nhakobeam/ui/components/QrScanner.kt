// Scanning a pairing code with the camera (CameraX), read with ZXing (no Google Play services).
package io.github.kimzam30.nhakobeam.ui.components

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.PlanarYUVLuminanceSource
import com.google.zxing.common.HybridBinarizer
import com.google.zxing.qrcode.QRCodeReader
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.style
import java.util.concurrent.Executors

/** The three words in a code link (`nhakobeam://pair/river-lantern-seven`), or null. */
fun wordsFromCodeUri(text: String): List<String>? {
    val m = Regex("""^nhakobeam://pair/([a-z]+)-([a-z]+)-([a-z]+)$""").find(text.trim().lowercase()) ?: return null
    return m.groupValues.drop(1)
}

@Composable
fun QrScanner(onFound: (List<String>) -> Unit, modifier: Modifier = Modifier) {
    val c = Nhako.colors
    val context = LocalContext.current
    var granted by remember { mutableStateOf(ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) }
    var asked by remember { mutableStateOf(false) }
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {
        granted = it
        asked = true
    }
    LaunchedEffect(Unit) { if (!granted) launcher.launch(Manifest.permission.CAMERA) }

    Box(modifier.fillMaxWidth().aspectRatio(1f).clip(RoundedCornerShape(NhakoRadius.xl)).background(Color(0xFF0D0E11)), contentAlignment = Alignment.Center) {
        if (granted) {
            CameraPreview(onFound)
            Viewfinder()
        } else if (asked) {
            Column(Modifier.padding(NhakoSpace.s7), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(NhakoSpace.s4)) {
                BasicText(t("code.scan.cameraDenied"), style = NhakoType.callout.style(Color.White).copy(textAlign = TextAlign.Center))
                NButton(t("code.scan.openSettings"), {
                    context.startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.packageName, null)))
                }, variant = ButtonVariant.Secondary, size = ButtonSize.Sm)
            }
        }
    }
}

@Composable
private fun Viewfinder() {
    val sweep = rememberInfiniteTransition(label = "scan").animateFloat(0.1f, 0.9f, infiniteRepeatable(tween(1800, easing = LinearEasing), androidx.compose.animation.core.RepeatMode.Reverse), label = "line")
    val reduced = Nhako.reducedMotion
    val accent = Nhako.colors.accent
    Canvas(Modifier.fillMaxSize().padding(36.dp)) {
        val corner = 28.dp.toPx()
        val len = 34.dp.toPx()
        val w = 4.dp.toPx()
        val s = size
        for ((x, y) in listOf(0f to 0f, s.width to 0f, 0f to s.height, s.width to s.height)) {
            val dx = if (x == 0f) 1 else -1
            val dy = if (y == 0f) 1 else -1
            drawLine(Color.White, Offset(x + dx * corner * 0.3f, y), Offset(x + dx * len, y), w, StrokeCap.Round)
            drawLine(Color.White, Offset(x, y + dy * corner * 0.3f), Offset(x, y + dy * len), w, StrokeCap.Round)
        }
        if (!reduced) {
            val ly = s.height * sweep.value
            drawRoundRect(accent.copy(alpha = 0.8f), Offset(len * 0.6f, ly), Size(s.width - len * 1.2f, 3.dp.toPx()), CornerRadius(2.dp.toPx()))
        }
        drawRoundRect(Color.White.copy(alpha = 0.12f), Offset.Zero, s, CornerRadius(corner), style = Stroke(1.dp.toPx()))
    }
}

@Composable
private fun CameraPreview(onFound: (List<String>) -> Unit) {
    val context = LocalContext.current
    val owner = LocalLifecycleOwner.current
    val executor = remember { Executors.newSingleThreadExecutor() }
    var done by remember { mutableStateOf(false) }
    DisposableEffect(Unit) { onDispose { executor.shutdown() } }
    AndroidView(
        modifier = Modifier.fillMaxSize(),
        factory = { ctx ->
            val view = PreviewView(ctx).apply { scaleType = PreviewView.ScaleType.FILL_CENTER }
            val providerFuture = ProcessCameraProvider.getInstance(ctx)
            providerFuture.addListener({
                val provider = providerFuture.get()
                val preview = Preview.Builder().build().also { it.surfaceProvider = view.surfaceProvider }
                val reader = QRCodeReader()
                val hints = mapOf(DecodeHintType.TRY_HARDER to true)
                val analysis = ImageAnalysis.Builder().setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST).build()
                analysis.setAnalyzer(executor) { image ->
                    image.use {
                        if (done) return@use
                        val plane = it.planes[0]
                        val buf = plane.buffer
                        val bytes = ByteArray(buf.remaining()).also { b -> buf.get(b) }
                        val source = PlanarYUVLuminanceSource(bytes, plane.rowStride, it.height, 0, 0, it.width, it.height, false)
                        val text = runCatching { reader.decode(BinaryBitmap(HybridBinarizer(source)), hints).text }.getOrNull()
                        val words = text?.let(::wordsFromCodeUri)
                        if (words != null) {
                            done = true
                            ContextCompat.getMainExecutor(context).execute { onFound(words) }
                        }
                    }
                }
                runCatching {
                    provider.unbindAll()
                    provider.bindToLifecycle(owner, CameraSelector.DEFAULT_BACK_CAMERA, preview, analysis)
                }
            }, ContextCompat.getMainExecutor(ctx))
            view
        },
        onRelease = { runCatching { ProcessCameraProvider.getInstance(context).get().unbindAll() } },
    )
}

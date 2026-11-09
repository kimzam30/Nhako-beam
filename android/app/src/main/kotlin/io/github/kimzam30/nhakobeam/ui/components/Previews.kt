package io.github.kimzam30.nhakobeam.ui.components

import android.graphics.BitmapFactory
import android.os.Build
import android.util.Base64
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.FileItem
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.formatBytes
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style

private const val TILES = 4
/** The list never draws thousands of rows. */
private const val LIST_MAX = 50

/** A preview's picture, from its `data:image/jpeg;base64,` URL. Null if it isn't one or won't decode. */
fun thumbBitmap(url: String?): ImageBitmap? {
    val b64 = url?.removePrefix("data:image/jpeg;base64,")?.takeIf { it != url } ?: return null
    val bytes = runCatching { Base64.decode(b64, Base64.DEFAULT) }.getOrNull() ?: return null
    // The engine caps a preview at 32 KiB; anything bigger never reached here.
    if (bytes.size > 32 * 1024) return null
    return runCatching { BitmapFactory.decodeByteArray(bytes, 0, bytes.size)?.asImageBitmap() }.getOrNull()
}

/**
 * Small previews of the photos and videos in a request (or staged to send): up to 4 tiles, the
 * last counting the rest, and the whole list on demand. From a device you haven't trusted they
 * show blurred until you tap. The desktop draws the same (Previews.svelte).
 */
@Composable
fun Previews(files: List<FileItem>, modifier: Modifier = Modifier, size: Dp = 76.dp, blurred: Boolean = false, list: Boolean = true) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    val tiles = remember(files) { files.filter { it.thumb != null }.take(TILES).mapNotNull { f -> thumbBitmap(f.thumb)?.let { f to it } } }
    if (tiles.isEmpty()) return
    val rest = files.size - tiles.size
    var open by remember { mutableStateOf(false) }
    var revealed by remember { mutableStateOf(false) }
    val hidden = blurred && !revealed
    // Blur needs Android 12; before it a stranger's previews stay covered instead.
    val canBlur = Build.VERSION.SDK_INT >= 31
    Column(modifier, verticalArrangement = Arrangement.spacedBy(NhakoSpace.s2)) {
        Box {
            Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s2)) {
                tiles.forEachIndexed { i, (f, bmp) ->
                    val shown by animateFloatAsState(1f, if (reduced) tween(1) else tween(180, delayMillis = i * 40), label = "tile")
                    Box(
                        Modifier.size(size).graphicsLayer { alpha = shown; translationY = (1 - shown) * 6.dp.toPx() }
                            .clip(RoundedCornerShape(NhakoRadius.md)).background(c.surfaceSunken)
                            .semantics { contentDescription = if (hidden) "" else t("preview.label", "file" to f.name) },
                    ) {
                        Image(
                            bmp, null,
                            Modifier.fillMaxSize().then(if (hidden && canBlur) Modifier.blur(10.dp) else Modifier),
                            contentScale = ContentScale.Crop,
                        )
                        if (hidden && !canBlur) Box(Modifier.fillMaxSize().background(c.surfaceSunken))
                        if (f.mime.startsWith("video/")) {
                            Box(Modifier.align(Alignment.BottomStart).padding(6.dp).size(24.dp).clip(CircleShape).background(Color(0x8C000000)), contentAlignment = Alignment.Center) {
                                PhIcon("play", size = 14.dp, tint = Color.White, weight = IconWeight.Fill)
                            }
                        }
                        if (i == tiles.lastIndex && rest > 0 && !hidden) {
                            Box(Modifier.fillMaxSize().background(Color(0xA3000000)), contentAlignment = Alignment.Center) {
                                BasicText("+$rest", style = NhakoType.headline.style(Color.White))
                            }
                        }
                    }
                }
            }
            if (hidden) {
                Box(
                    Modifier.matchParentSize().clip(RoundedCornerShape(NhakoRadius.md)).clickable(role = Role.Button) { revealed = true },
                    contentAlignment = Alignment.Center,
                ) {
                    Row(
                        Modifier.clip(CircleShape).background(Color(0x9E000000)).padding(horizontal = NhakoSpace.s3, vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s2),
                    ) {
                        PhIcon("eye", size = 16.dp, tint = Color.White)
                        BasicText(t("preview.blurred"), style = NhakoType.footnote.style(Color.White).copy(fontWeight = FontWeight(600)), maxLines = 1)
                    }
                }
            }
        }
        if (list && files.size > 1) {
            val turn by animateFloatAsState(if (open) 90f else 0f, if (reduced) tween(1) else tween(NhakoMotion.Duration.fast), label = "chev")
            Row(
                Modifier.heightIn(min = 48.dp).clip(RoundedCornerShape(NhakoRadius.sm)).clickable(role = Role.Button) { open = !open }
                    .semantics { stateDescription = if (open) "expanded" else "collapsed" }.padding(horizontal = NhakoSpace.s1),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s1),
            ) {
                BasicText(t(if (open) "preview.hide" else "preview.show", "count" to files.size), style = NhakoType.footnote.style(c.accentText).copy(fontWeight = FontWeight(600)))
                PhIcon("caret-right", Modifier.graphicsLayer { rotationZ = turn }, size = 12.dp, tint = c.accentText, weight = IconWeight.Bold)
            }
            AnimatedVisibility(open, enter = expandVertically(NhakoMotion.smooth.spec()) + fadeIn(), exit = shrinkVertically(tween(NhakoMotion.Duration.fast)) + fadeOut()) {
                Column {
                    for (f in files.take(LIST_MAX)) {
                        androidx.compose.runtime.key(f.id) {
                            Row(Modifier.fillMaxWidth().heightIn(min = 40.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                                val bmp = remember(f.thumb) { if (hidden) null else thumbBitmap(f.thumb) }
                                Box(Modifier.size(32.dp).clip(RoundedCornerShape(NhakoRadius.sm)).background(c.surfaceSunken), contentAlignment = Alignment.Center) {
                                    if (bmp != null && !hidden) Image(bmp, null, Modifier.fillMaxSize(), contentScale = ContentScale.Crop)
                                    else PhIcon(if (f.mime.startsWith("video/")) "film-strip" else if (f.mime.startsWith("image/")) "image" else "file", size = 16.dp, tint = c.textSecondary)
                                }
                                BasicText(listOfNotNull(f.folder, f.name).joinToString("/"), Modifier.weight(1f), style = NhakoType.footnote.style(c.textPrimary), maxLines = 1, overflow = TextOverflow.Ellipsis)
                                BasicText(formatBytes(f.size), style = NhakoType.footnote.style(c.textSecondary))
                            }
                        }
                    }
                    if (files.size > LIST_MAX) BasicText(t("preview.more", "count" to files.size - LIST_MAX), style = NhakoType.footnote.style(c.textSecondary))
                }
            }
        }
    }
}

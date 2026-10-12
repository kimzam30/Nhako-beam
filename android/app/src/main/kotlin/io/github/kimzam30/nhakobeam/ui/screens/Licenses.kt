// About > Open source licenses: every package the app ships, with its license text. The list is
// made by scripts/licenses.mjs (assets/licenses.json) and read when this screen opens.
// The Compose twin of desktop/src/screens/Licenses.svelte.
package io.github.kimzam30.nhakobeam.ui.screens

import android.content.Context
import android.content.Intent
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.net.toUri
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.Screen
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.components.NIconButton
import io.github.kimzam30.nhakobeam.ui.components.NTextField
import io.github.kimzam30.nhakobeam.ui.components.Separator
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import io.github.kimzam30.nhakobeam.ui.withTap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

@Serializable
data class LicensedPackage(val name: String, val version: String, val license: String, val url: String, val ecosystem: String, val texts: List<String>)

@Serializable
data class Licenses(val packages: List<LicensedPackage>, val texts: Map<String, String>)

private val json = Json { ignoreUnknownKeys = true }

fun readLicenses(context: Context): Licenses =
    context.assets.open("licenses.json").use { json.decodeFromString(Licenses.serializer(), it.readBytes().decodeToString()) }

@Composable
fun LicensesScreen(app: AppState) {
    val c = Nhako.colors
    val context = LocalContext.current
    BackHandler { app.navigate(Screen.Settings) }
    val data by produceState<Licenses?>(null) { value = withContext(Dispatchers.IO) { runCatching { readLicenses(context) }.getOrNull() } }
    var query by rememberSaveable { mutableStateOf("") }
    var open by rememberSaveable { mutableStateOf<String?>(null) }
    val shown = remember(data, query) {
        val q = query.trim().lowercase()
        data?.packages.orEmpty().filter { q.isEmpty() || q in it.name.lowercase() || q in it.license.lowercase() }
    }
    val appear by animateFloatAsState(if (data == null) 0f else 1f, if (Nhako.reducedMotion) tween(NhakoMotion.Duration.fast) else NhakoMotion.smooth.spec(), label = "list")

    Column(Modifier.fillMaxSize().background(c.canvas).windowInsetsPadding(WindowInsets.statusBars).windowInsetsPadding(WindowInsets.ime)) {
        Row(Modifier.padding(start = NhakoSpace.s3, end = NhakoSpace.s4, top = NhakoSpace.s3), verticalAlignment = Alignment.CenterVertically) {
            NIconButton("caret-left", t("settings.back"), { app.navigate(Screen.Settings) }, size = 24.dp, weight = IconWeight.Bold)
            BasicText(t("licenses.title"), style = NhakoType.title1.style(c.textPrimary))
        }
        LazyColumn(
            Modifier.fillMaxSize().graphicsLayer { alpha = appear; translationY = (1 - appear) * 12.dp.toPx() },
            contentPadding = PaddingValues(
                start = NhakoSpace.s5, end = NhakoSpace.s5, top = NhakoSpace.s4,
                bottom = NhakoSpace.s9 + WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding(),
            ),
        ) {
            item(key = "intro") {
                Column(verticalArrangement = Arrangement.spacedBy(NhakoSpace.s4), modifier = Modifier.padding(bottom = NhakoSpace.s4)) {
                    BasicText(t("licenses.intro"), style = NhakoType.callout.style(c.textSecondary))
                    NTextField(
                        query, { query = it }, Modifier.fillMaxWidth(), placeholder = t("licenses.search"),
                        style = NhakoType.callout.style(c.textPrimary),
                        trailing = { BasicText(t("licenses.count", "count" to shown.size), Modifier.padding(end = NhakoSpace.s3), style = NhakoType.footnote.style(c.textTertiary)) },
                    )
                }
            }
            items(shown, key = { "${it.ecosystem}:${it.name}@${it.version}" }) { p ->
                val key = "${p.ecosystem}:${p.name}@${p.version}"
                val isOpen = open == key
                Column(Modifier.animateItem().clip(RoundedCornerShape(NhakoRadius.md)).background(c.surface)) {
                    Row(
                        Modifier.fillMaxWidth().clickable(role = Role.Button, onClick = withTap { open = if (isOpen) null else key }).padding(horizontal = NhakoSpace.s5, vertical = NhakoSpace.s3),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(Modifier.weight(1f)) {
                            BasicText(p.name, style = NhakoType.body.style(c.textPrimary).copy(fontWeight = FontWeight(500)))
                            BasicText("${p.version} · ${p.license}", style = NhakoType.footnote.style(c.textSecondary))
                        }
                        val turn by animateFloatAsState(if (isOpen) 90f else 0f, NhakoMotion.snappy.spec(), label = "chevron")
                        PhIcon("caret-right", Modifier.graphicsLayer { rotationZ = turn }, size = 16.dp, tint = c.textTertiary)
                    }
                    AnimatedVisibility(isOpen, enter = expandVertically(NhakoMotion.smooth.spec()) + fadeIn(), exit = shrinkVertically(tween(NhakoMotion.Duration.fast)) + fadeOut()) {
                        Column(Modifier.padding(start = NhakoSpace.s5, end = NhakoSpace.s5, bottom = NhakoSpace.s4), verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                            BasicText(
                                p.url,
                                Modifier.clickable(role = Role.Button, onClick = withTap { runCatching { context.startActivity(Intent(Intent.ACTION_VIEW, p.url.toUri())) } }),
                                style = NhakoType.footnote.style(c.accentText),
                            )
                            p.texts.forEach { id ->
                                BasicText(
                                    data?.texts?.get(id).orEmpty(),
                                    Modifier.fillMaxWidth().clip(RoundedCornerShape(NhakoRadius.sm)).background(c.surfaceSunken).horizontalScroll(rememberScrollState()).padding(NhakoSpace.s4),
                                    style = NhakoType.caption.style(c.textSecondary).copy(fontFamily = FontFamily.Monospace, fontSize = 11.sp, lineHeight = 16.sp),
                                )
                            }
                        }
                    }
                    Separator()
                }
            }
        }
    }
}

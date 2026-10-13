package io.github.kimzam30.nhakobeam.ui.screens

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoHaptic
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.CheckResult
import io.github.kimzam30.nhakobeam.model.Connection
import io.github.kimzam30.nhakobeam.model.ServerCheck
import io.github.kimzam30.nhakobeam.model.WhoCanSend
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.MotionPref
import io.github.kimzam30.nhakobeam.state.Prefs
import io.github.kimzam30.nhakobeam.state.Screen
import io.github.kimzam30.nhakobeam.state.SoundPref
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.state.ThemePref
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.OwnForm
import io.github.kimzam30.nhakobeam.ui.components.ButtonSize
import io.github.kimzam30.nhakobeam.ui.components.ButtonVariant
import io.github.kimzam30.nhakobeam.ui.components.DeviceOrb
import io.github.kimzam30.nhakobeam.ui.components.NButton
import io.github.kimzam30.nhakobeam.ui.components.NCard
import io.github.kimzam30.nhakobeam.ui.components.NIconButton
import io.github.kimzam30.nhakobeam.ui.components.NSegmented
import io.github.kimzam30.nhakobeam.ui.components.NTextField
import io.github.kimzam30.nhakobeam.ui.components.NToggle
import io.github.kimzam30.nhakobeam.ui.components.SectionTitle
import io.github.kimzam30.nhakobeam.ui.components.SegmentOption
import io.github.kimzam30.nhakobeam.ui.components.pressScale
import io.github.kimzam30.nhakobeam.ui.components.Separator
import io.github.kimzam30.nhakobeam.ui.hostOf
import io.github.kimzam30.nhakobeam.ui.readOwnForm
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import io.github.kimzam30.nhakobeam.ui.withTap
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@Composable
fun SettingsScreen(app: AppState, prefs: Prefs) {
    val c = Nhako.colors
    BackHandler { app.navigate(Screen.Home) }
    val trusted = app.devices.filter { it.trusted }.sortedBy { it.name }
    val nearbyUntrusted = app.devices.filter { !it.trusted && it.reachable && it.route == io.github.kimzam30.nhakobeam.model.Route.Nearby }.sortedBy { it.name }

    Column(
        Modifier.fillMaxSize().background(c.canvas).windowInsetsPadding(WindowInsets.statusBars).windowInsetsPadding(WindowInsets.ime)
            .verticalScroll(rememberScrollState()).windowInsetsPadding(WindowInsets.navigationBars)
            .padding(horizontal = NhakoSpace.s5).padding(top = NhakoSpace.s3, bottom = NhakoSpace.s9),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            NIconButton("caret-left", t("settings.back"), { app.navigate(Screen.Home) }, size = 24.dp, weight = IconWeight.Bold)
            BasicText(t("settings.title"), style = NhakoType.title1.style(c.textPrimary))
        }
        Spacer(Modifier.height(NhakoSpace.s6))
        Column(Modifier.widthIn(max = 640.dp), verticalArrangement = Arrangement.spacedBy(NhakoSpace.s7)) {
            Section(0, t("settings.section.device")) {
                NCard {
                    Column {
                        Row(Modifier.fillMaxWidth().padding(vertical = NhakoSpace.s3), verticalAlignment = Alignment.CenterVertically) {
                            BasicText(t("settings.device.name"), style = NhakoType.body.style(c.textPrimary))
                            Spacer(Modifier.width(NhakoSpace.s5))
                            var name by remember { mutableStateOf(app.selfName) }
                            NTextField(name, { name = it.take(32) }, Modifier.weight(1f), align = androidx.compose.ui.text.style.TextAlign.End, capitalize = true, onDone = { if (name.isNotBlank()) app.renameSelf(name) })
                            LaunchedEffect(name) {
                                delay(800)
                                if (name.isNotBlank() && name != app.selfName) app.renameSelf(name)
                            }
                        }
                        Separator()
                        Row(Modifier.fillMaxWidth().padding(vertical = NhakoSpace.s4), verticalAlignment = Alignment.CenterVertically) {
                            BasicText(t("settings.device.save"), style = NhakoType.body.style(c.textPrimary))
                            Spacer(Modifier.weight(1f))
                            PhIcon("folder-open", size = 18.dp, tint = c.textSecondary)
                            Spacer(Modifier.width(NhakoSpace.s2))
                            BasicText("Download/Nhako Beam", style = NhakoType.callout.style(c.textSecondary))
                        }
                    }
                }
            }

            Section(1, t("settings.section.receiving")) {
                NCard(padding = androidx.compose.foundation.layout.PaddingValues(NhakoSpace.s5)) { WhoCanSendSetting(app) }
            }

            Section(2, t("settings.trusted.title")) {
                NCard {
                    Column {
                        if (trusted.isEmpty()) {
                            BasicText(t("settings.trusted.empty"), Modifier.padding(vertical = NhakoSpace.s4), style = NhakoType.callout.style(c.textSecondary))
                        }
                        trusted.forEachIndexed { i, d ->
                            if (i > 0) Separator()
                            Row(Modifier.fillMaxWidth().padding(top = NhakoSpace.s3), verticalAlignment = Alignment.CenterVertically) {
                                DeviceOrb(d, 40.dp, showName = false)
                                Spacer(Modifier.width(NhakoSpace.s3))
                                Column(Modifier.weight(1f)) {
                                    BasicText(d.name, style = NhakoType.headline.style(c.textPrimary), maxLines = 1, overflow = TextOverflow.Ellipsis)
                                    BasicText(d.fingerprint, style = NhakoType.footnote.style(c.textSecondary))
                                }
                                NIconButton("trash", "${t("settings.trusted.remove")} ${d.name}", { app.sheet = Sheet.ConfirmRemove(d.id) }, tint = c.danger)
                            }
                            NToggle(d.autoAccept, t("settings.trusted.autoAccept"), { app.engine.setAutoAccept(d.id, it) })
                        }
                    }
                }
                // Devices here right now that aren't yours yet: trusting one asks the other to agree.
                if (nearbyUntrusted.isNotEmpty()) {
                    Spacer(Modifier.height(NhakoSpace.s5))
                    SectionTitle(t("settings.trusted.nearby"))
                    NCard {
                        Column {
                            nearbyUntrusted.forEachIndexed { i, d ->
                                if (i > 0) Separator()
                                Row(Modifier.fillMaxWidth().padding(vertical = NhakoSpace.s3), verticalAlignment = Alignment.CenterVertically) {
                                    DeviceOrb(d, 40.dp, showName = false)
                                    Spacer(Modifier.width(NhakoSpace.s3))
                                    Column(Modifier.weight(1f)) {
                                        BasicText(d.name, style = NhakoType.headline.style(c.textPrimary), maxLines = 1, overflow = TextOverflow.Ellipsis)
                                        BasicText(d.fingerprint, style = NhakoType.footnote.style(c.textSecondary))
                                    }
                                    NButton(t("send.choose.trust"), { app.askTrust(d.id) }, variant = ButtonVariant.Secondary, size = ButtonSize.Sm, icon = "handshake")
                                }
                            }
                        }
                    }
                }
            }

            Section(3, t("settings.section.connection")) {
                Column(verticalArrangement = Arrangement.spacedBy(NhakoSpace.s4)) {
                    NCard {
                        val scope = rememberCoroutineScope()
                        // Flips at once; goes back if the engine couldn't restart (transfers running).
                        var lan by remember(app.lanDiscovery) { mutableStateOf(app.lanDiscovery) }
                        NToggle(lan, t("settings.network.lan"), { on ->
                            lan = on
                            scope.launch { if (!app.setLanDiscovery(on)) lan = !on }
                        }, hint = t("settings.network.lan.hint"))
                    }
                    NCard(padding = androidx.compose.foundation.layout.PaddingValues(NhakoSpace.s5)) { ConnectionSettings(app) }
                }
            }

            Section(4, t("settings.section.appearance")) {
                NCard(padding = androidx.compose.foundation.layout.PaddingValues(NhakoSpace.s5)) {
                    Column(verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                        BasicText(t("settings.appearance.theme"), style = NhakoType.body.style(c.textPrimary))
                        NSegmented(
                            listOf(
                                SegmentOption(ThemePref.System, t("settings.appearance.theme.system"), "circle-half"),
                                SegmentOption(ThemePref.Light, t("settings.appearance.theme.light"), "sun"),
                                SegmentOption(ThemePref.Dark, t("settings.appearance.theme.dark"), "moon"),
                            ),
                            prefs.theme, { prefs.theme = it },
                        )
                        Spacer(Modifier.height(NhakoSpace.s3))
                        BasicText(t("settings.appearance.motion"), style = NhakoType.body.style(c.textPrimary))
                        NSegmented(
                            listOf(
                                SegmentOption(MotionPref.System, t("settings.appearance.motion.system")),
                                SegmentOption(MotionPref.Full, t("settings.appearance.motion.full")),
                                SegmentOption(MotionPref.Reduced, t("settings.appearance.motion.reduced")),
                            ),
                            prefs.motion, { prefs.motion = it },
                        )
                        BasicText(t("settings.appearance.motion.hint"), style = NhakoType.footnote.style(c.textSecondary))
                        Spacer(Modifier.height(NhakoSpace.s3))
                        BasicText(t("settings.appearance.sounds"), style = NhakoType.body.style(c.textPrimary))
                        NSegmented(
                            listOf(
                                SegmentOption(SoundPref.Loud, t("settings.appearance.sounds.loud"), "speaker-high"),
                                SegmentOption(SoundPref.Soft, t("settings.appearance.sounds.soft"), "speaker-low"),
                                SegmentOption(SoundPref.Off, t("settings.appearance.sounds.off"), "speaker-slash"),
                            ),
                            prefs.sounds, { prefs.sounds = it },
                        )
                        BasicText(t("settings.appearance.sounds.hint"), style = NhakoType.footnote.style(c.textSecondary))
                    }
                }
            }

            Section(5, t("settings.about.title")) {
                NCard {
                    Column {
                        Row(Modifier.fillMaxWidth().padding(vertical = NhakoSpace.s4)) {
                            BasicText(t("settings.about.version", "version" to app.version.ifBlank { "0.1.0" }), style = NhakoType.body.style(c.textPrimary))
                        }
                        app.fingerprint?.let {
                            Separator()
                            Row(Modifier.fillMaxWidth().padding(vertical = NhakoSpace.s4)) {
                                BasicText(t("settings.about.fingerprint"), Modifier.weight(1f), style = NhakoType.body.style(c.textPrimary))
                                BasicText(it, style = NhakoType.body.style(c.textSecondary))
                            }
                        }
                        Separator()
                        Column(Modifier.fillMaxWidth().padding(vertical = NhakoSpace.s4), verticalArrangement = Arrangement.spacedBy(NhakoSpace.s1)) {
                            Row {
                                BasicText(t("settings.about.wake"), Modifier.weight(1f), style = NhakoType.body.style(c.textPrimary))
                                BasicText(t(if (app.wakeReady) "settings.about.wake.on" else "settings.about.wake.off"), style = NhakoType.body.style(if (app.wakeReady) c.success else c.textSecondary))
                            }
                            BasicText(t(if (app.wakeReady) "settings.about.wake.on.hint" else "settings.about.wake.off.hint"), style = NhakoType.footnote.style(c.textSecondary))
                        }
                        Separator()
                        val press = remember { androidx.compose.foundation.interaction.MutableInteractionSource() }
                        Row(
                            Modifier.fillMaxWidth().pressScale(press).clickable(press, null, onClick = withTap { app.navigate(Screen.Licenses) }).padding(vertical = NhakoSpace.s4),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            BasicText(t("settings.about.licenses"), Modifier.weight(1f), style = NhakoType.body.style(c.textPrimary))
                            PhIcon("caret-right", size = 16.dp, tint = c.textTertiary)
                        }
                        Separator()
                        DiagnosticsRow(app)
                    }
                }
            }
        }
    }
}

/** Settings > About > Copy diagnostics. The copy icon turns into a check once it's on the clipboard. */
@Composable
private fun DiagnosticsRow(app: AppState) {
    val c = Nhako.colors
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val press = remember { androidx.compose.foundation.interaction.MutableInteractionSource() }
    var copied by remember { mutableStateOf(false) }
    Row(
        Modifier.fillMaxWidth().pressScale(press).clickable(press, null, onClick = withTap {
            scope.launch {
                val ok = app.copyDiagnostics { text ->
                    val cm = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                    cm.setPrimaryClip(ClipData.newPlainText("Nhako Beam diagnostics", text))
                }
                if (ok) {
                    copied = true
                    delay(1600)
                    copied = false
                }
            }
        }).padding(vertical = NhakoSpace.s4),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s4),
    ) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(NhakoSpace.s1)) {
            BasicText(t("settings.about.diagnostics"), style = NhakoType.body.style(c.textPrimary))
            BasicText(t("settings.about.diagnostics.hint"), style = NhakoType.footnote.style(c.textSecondary))
        }
        val reduced = Nhako.reducedMotion
        AnimatedContent(
            copied,
            transitionSpec = {
                if (reduced) {
                    fadeIn(tween(NhakoMotion.Duration.fast)) togetherWith fadeOut(tween(NhakoMotion.Duration.fast))
                } else {
                    (fadeIn(tween(NhakoMotion.Duration.fast)) + scaleIn(NhakoMotion.snappy.spec(), initialScale = 0.6f)) togetherWith
                        (fadeOut(tween(90)) + scaleOut(tween(90), targetScale = 0.6f))
                }
            },
            label = "diagnostics icon",
        ) { done ->
            PhIcon(if (done) "check" else "copy", size = 18.dp, tint = if (done) c.success else c.textTertiary)
        }
    }
}

/** Settings > Receiving > Who can send to me, with a hint that changes with the choice. */
@Composable
private fun WhoCanSendSetting(app: AppState) {
    val c = Nhako.colors
    val scope = rememberCoroutineScope()
    val reduced = Nhako.reducedMotion
    Column(verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
        BasicText(t("settings.receive.who"), style = NhakoType.body.style(c.textPrimary))
        NSegmented(
            listOf(
                SegmentOption(WhoCanSend.YourDevices, t("settings.receive.who.yours"), "shield-check"),
                SegmentOption(WhoCanSend.EveryoneNearby, t("settings.receive.who.everyone"), "broadcast"),
            ),
            app.whoCanSend, { who -> scope.launch { app.setWhoCanSend(who) } },
        )
        AnimatedContent(
            app.whoCanSend,
            transitionSpec = {
                if (reduced) {
                    fadeIn(tween(NhakoMotion.Duration.fast)) togetherWith fadeOut(tween(NhakoMotion.Duration.fast))
                } else {
                    (fadeIn(tween(NhakoMotion.Duration.fast)) + slideInVertically(NhakoMotion.snappy.spec()) { -it / 4 }) togetherWith fadeOut(tween(90))
                }
            },
            label = "who can send hint",
        ) { who ->
            BasicText(
                t(if (who == WhoCanSend.YourDevices) "settings.receive.who.yours.hint" else "settings.receive.who.everyone.hint"),
                style = NhakoType.footnote.style(c.textSecondary),
            )
        }
    }
}

/** A settings section that rises in, staggered by [order]. */
@Composable
private fun Section(order: Int, title: String, content: @Composable () -> Unit) {
    val reduced = Nhako.reducedMotion
    val enter = remember { Animatable(if (reduced) 1f else 0f) }
    LaunchedEffect(Unit) {
        delay((order.coerceAtMost(5) * NhakoMotion.staggerStepMs).toLong())
        enter.animateTo(1f, NhakoMotion.smooth.spec())
    }
    Column(Modifier.graphicsLayer { translationY = (1 - enter.value) * 12.dp.toPx(); alpha = enter.value.coerceIn(0f, 1f) }) {
        SectionTitle(title)
        content()
    }
}

/** Settings > Connection: n0's public servers, or your own (the Docker setup in relay/). */
@Composable
private fun ConnectionSettings(app: AppState) {
    val c = Nhako.colors
    val scope = rememberCoroutineScope()
    val view = androidx.compose.ui.platform.LocalView.current
    val saved = app.connection
    var own by remember { mutableStateOf(saved is Connection.Own) }
    fun formFrom() = (app.connection as? Connection.Own)?.let { OwnForm(it.relay, it.token.orEmpty(), it.lookup.orEmpty()) } ?: OwnForm()
    var form by remember { mutableStateOf(formFrom()) }
    var touched by remember { mutableStateOf(false) }
    var showToken by remember { mutableStateOf(false) }
    var checking by remember { mutableStateOf(false) }
    var saving by remember { mutableStateOf(false) }
    var check by remember { mutableStateOf<ServerCheck?>(null) }
    var checkRun by remember { mutableIntStateOf(0) }
    var askAnyway by remember { mutableStateOf(false) }
    val read = readOwnForm(form)
    val isSavedOwn = saved is Connection.Own && read.connection != null &&
        saved.relay == read.connection.relay && (saved.token ?: "") == (read.connection.token ?: "") && (saved.lookup ?: "") == (read.connection.lookup ?: "")

    fun edited(f: OwnForm) {
        form = f
        check = null
        askAnyway = false
    }

    fun allOk(s: ServerCheck) = s.relay is CheckResult.Ok && (s.lookup == null || s.lookup is CheckResult.Ok)

    suspend fun runCheck(): ServerCheck? {
        touched = true
        val conn = read.connection ?: return null
        checking = true
        askAnyway = false
        val result = app.engine.checkServer(conn)
        check = result
        checkRun++
        (if (allOk(result)) NhakoHaptic.Accept else NhakoHaptic.TransferFailed).perform(view)
        checking = false
        return result
    }

    fun commit() {
        val conn = read.connection ?: return
        scope.launch {
            saving = true
            if (app.setConnection(conn)) askAnyway = false
            saving = false
        }
    }

    Column(verticalArrangement = Arrangement.spacedBy(NhakoSpace.s4)) {
        BasicText(t("settings.connection.servers"), style = NhakoType.body.style(c.textPrimary))
        NSegmented(
            listOf(SegmentOption(false, t("settings.connection.public"), "globe-hemisphere-west"), SegmentOption(true, t("settings.connection.own"), "hard-drives")),
            own,
            { mode ->
                own = mode
                check = null
                askAnyway = false
                if (!mode && saved is Connection.Own) scope.launch {
                    saving = true
                    if (!app.setConnection(Connection.Public)) own = true
                    saving = false
                }
                if (mode) {
                    form = formFrom()
                    touched = false
                }
            },
        )
        AnimatedContent(own, transitionSpec = { fadeIn(tween(NhakoMotion.Duration.base)) togetherWith fadeOut(tween(NhakoMotion.Duration.fast)) }, label = "hint") { o ->
            BasicText(t(if (o) "settings.connection.own.hint" else "settings.connection.public.hint"), style = NhakoType.footnote.style(c.textSecondary))
        }
        AnimatedVisibility(own, enter = expandVertically(NhakoMotion.smooth.spec()) + fadeIn(), exit = shrinkVertically(tween(NhakoMotion.Duration.fast)) + fadeOut()) {
            Column(verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                Separator()
                if (isSavedOwn && check == null) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Box(Modifier.size(8.dp).clip(CircleShape).background(c.success))
                        Spacer(Modifier.width(NhakoSpace.s3))
                        BasicText(t("settings.connection.using", "host" to hostOf(form.relay)), style = NhakoType.callout.style(c.textPrimary).copy(fontWeight = FontWeight(600)))
                    }
                }
                Field(t("settings.connection.relay"), (read.relay?.code).takeIf { touched }) {
                    NTextField(form.relay, { edited(form.copy(relay = it)) }, Modifier.fillMaxWidth(), placeholder = t("settings.connection.relay.placeholder"), error = touched && read.relay != null, style = NhakoType.callout.style(c.textPrimary))
                }
                Field(t("settings.connection.token"), (read.token?.code).takeIf { touched }) {
                    NTextField(
                        form.token, { edited(form.copy(token = it)) }, Modifier.fillMaxWidth(), placeholder = t("settings.connection.token.placeholder"), secret = !showToken,
                        error = touched && read.token != null, style = NhakoType.callout.style(c.textPrimary),
                        trailing = { NIconButton(if (showToken) "eye-slash" else "eye", t(if (showToken) "settings.connection.token.hide" else "settings.connection.token.show"), { showToken = !showToken }, size = 20.dp) },
                    )
                }
                Field(t("settings.connection.lookup"), (read.lookup?.code).takeIf { touched }, hint = t("settings.connection.lookup.hint")) {
                    NTextField(form.lookup, { edited(form.copy(lookup = it)) }, Modifier.fillMaxWidth(), placeholder = t("settings.connection.lookup.placeholder"), error = touched && read.lookup != null, style = NhakoType.callout.style(c.textPrimary))
                }
                check?.let { result ->
                    androidx.compose.runtime.key(checkRun) {
                        Column(Modifier.fillMaxWidth().clip(RoundedCornerShape(NhakoRadius.md)).background(c.surfaceSunken).padding(NhakoSpace.s4), verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                            ResultRow(0, t("settings.connection.result.relay"), result.relay, isRelay = true, tokenEmpty = form.token.isBlank())
                            result.lookup?.let { ResultRow(1, t("settings.connection.result.lookup"), it, isRelay = false, tokenEmpty = false) }
                        }
                    }
                    if (result.relay is CheckResult.TokenRejected) {
                        BasicText(t("settings.connection.notSaved.token"), style = NhakoType.footnote.style(c.danger))
                    }
                }
                if (askAnyway) {
                    Column(Modifier.fillMaxWidth().clip(RoundedCornerShape(NhakoRadius.md)).background(c.warningSubtle).padding(NhakoSpace.s4), verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                        BasicText(t("settings.connection.notReached"), style = NhakoType.footnote.style(c.textPrimary))
                        NButton(t("settings.connection.saveAnyway"), { commit() }, variant = ButtonVariant.Secondary, size = ButtonSize.Sm, loading = saving)
                    }
                }
                Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3, Alignment.End), modifier = Modifier.fillMaxWidth()) {
                    NButton(t("settings.connection.check"), { scope.launch { runCheck() } }, variant = ButtonVariant.Secondary, icon = "arrows-clockwise", loading = checking && !askAnyway)
                    NButton(t("settings.connection.save"), {
                        scope.launch {
                            val r = runCheck() ?: return@launch
                            if (r.relay is CheckResult.TokenRejected || r.relay is CheckResult.Invalid || r.lookup is CheckResult.Invalid) return@launch
                            if (allOk(r)) commit() else askAnyway = true
                        }
                    }, icon = "check", enabled = !isSavedOwn && !checking, loading = saving)
                }
            }
        }
    }
}

@Composable
private fun Field(label: String, problem: String?, hint: String? = null, content: @Composable () -> Unit) {
    val c = Nhako.colors
    Column(verticalArrangement = Arrangement.spacedBy(NhakoSpace.s2)) {
        BasicText(label, style = NhakoType.footnote.style(c.textSecondary).copy(fontWeight = FontWeight(600)))
        content()
        AnimatedVisibility(problem != null, enter = expandVertically(NhakoMotion.snappy.spec()) + fadeIn(), exit = shrinkVertically(tween(NhakoMotion.Duration.fast)) + fadeOut()) {
            BasicText(problem?.let { t("server.problem.$it") }.orEmpty(), style = NhakoType.footnote.style(c.danger))
        }
        if (hint != null && problem == null) BasicText(hint, style = NhakoType.footnote.style(c.textSecondary))
    }
}

@Composable
private fun ResultRow(order: Int, label: String, r: CheckResult, isRelay: Boolean, tokenEmpty: Boolean) {
    val c = Nhako.colors
    val pop = remember { Animatable(0.6f) }
    LaunchedEffect(Unit) {
        delay(order * 60L)
        pop.animateTo(1f, NhakoMotion.bouncy.spec())
    }
    val (icon, tint) = when (r) {
        is CheckResult.Ok -> "check-circle" to c.success
        is CheckResult.Unreachable -> "warning-circle" to c.warning
        else -> "x-circle" to c.danger
    }
    val text = when (r) {
        is CheckResult.Ok -> t("settings.connection.result.ok", "ms" to r.ms)
        CheckResult.TokenRejected -> t(if (isRelay && tokenEmpty) "settings.connection.result.tokenNeeded" else "settings.connection.result.tokenRejected")
        is CheckResult.Unreachable -> t("settings.connection.result.unreachable")
        is CheckResult.Invalid -> t("settings.connection.result.invalid")
    }
    val why = when (r) {
        is CheckResult.Unreachable -> r.reason.replaceFirstChar { it.uppercase() }
        is CheckResult.Invalid -> r.reason.removePrefix("token: ").let { code -> runCatching { t("server.problem.$code") }.getOrDefault(code) }
        else -> null
    }
    Row(Modifier.graphicsLayer { scaleX = pop.value; scaleY = pop.value }, verticalAlignment = Alignment.Top) {
        PhIcon(icon, size = 18.dp, tint = tint, weight = IconWeight.Fill)
        Spacer(Modifier.width(NhakoSpace.s3))
        Column {
            Row {
                BasicText(label, style = NhakoType.callout.style(c.textPrimary).copy(fontWeight = FontWeight(600)))
                Spacer(Modifier.width(NhakoSpace.s3))
                BasicText(text, style = NhakoType.callout.style(c.textSecondary))
            }
            if (why != null) BasicText(why, style = NhakoType.footnote.style(c.textSecondary))
        }
    }
}

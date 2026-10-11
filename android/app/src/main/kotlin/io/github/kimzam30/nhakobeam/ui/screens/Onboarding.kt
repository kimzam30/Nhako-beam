package io.github.kimzam30.nhakobeam.ui.screens

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalInspectionMode
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.components.ButtonSize
import io.github.kimzam30.nhakobeam.ui.components.ButtonVariant
import io.github.kimzam30.nhakobeam.ui.components.DeviceOrb
import io.github.kimzam30.nhakobeam.ui.components.NButton
import io.github.kimzam30.nhakobeam.ui.components.NIconButton
import io.github.kimzam30.nhakobeam.ui.components.NTextField
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import kotlinx.coroutines.delay

private enum class Step { Welcome, Name, Nearby, Notify, Save }

/** The permission steps only exist where Android asks for those permissions (13 and newer). */
private val steps: List<Step> = buildList {
    add(Step.Welcome)
    add(Step.Name)
    if (Build.VERSION.SDK_INT >= 33) {
        add(Step.Nearby)
        add(Step.Notify)
    }
    add(Step.Save)
}

@Composable
fun OnboardingScreen(app: AppState) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    var index by rememberSaveable { mutableIntStateOf(0) }
    var dir by remember { mutableIntStateOf(1) }
    val step = steps[index.coerceIn(0, steps.lastIndex)]
    LaunchedEffect(Unit) { if (app.selfName.isBlank()) app.selfName = "My phone" }

    fun go(d: Int) {
        dir = d
        if (index + d > steps.lastIndex) app.finishOnboarding() else index = (index + d).coerceAtLeast(0)
    }

    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { go(1) }
    // Android 10 and older need the storage permission to save into Download.
    val storage = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { app.finishOnboarding() }
    val needsStorage = Build.VERSION.SDK_INT <= 29 && !LocalInspectionMode.current &&
        ContextCompat.checkSelfPermission(LocalContext.current, Manifest.permission.WRITE_EXTERNAL_STORAGE) != PackageManager.PERMISSION_GRANTED

    Column(
        Modifier.fillMaxSize().background(c.canvas).windowInsetsPadding(WindowInsets.statusBars).windowInsetsPadding(WindowInsets.navigationBars).windowInsetsPadding(WindowInsets.ime)
            .padding(horizontal = NhakoSpace.s6, vertical = NhakoSpace.s6),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        // Progress dots: the current one stretches into a pill.
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            steps.forEachIndexed { i, _ ->
                val w by animateDpAsState(if (i == index) 22.dp else 6.dp, NhakoMotion.snappy.spec(), label = "dot")
                Box(Modifier.size(w, 6.dp).clip(CircleShape).background(if (i <= index) c.accent else c.fillPressed))
            }
        }
        AnimatedContent(
            step,
            Modifier.weight(1f).fillMaxWidth(),
            transitionSpec = {
                if (reduced) fadeIn(tween(150)) togetherWith fadeOut(tween(150))
                else (slideInHorizontally(NhakoMotion.gentle.spec()) { dir * it / 14 } + fadeIn(tween(NhakoMotion.Duration.base))) togetherWith
                    (slideOutHorizontally(tween(NhakoMotion.Duration.fast)) { -dir * it / 40 } + fadeOut(tween(NhakoMotion.Duration.fast)))
            },
            label = "step",
        ) { s ->
            Column(Modifier.fillMaxSize().widthIn(max = 480.dp), verticalArrangement = Arrangement.Center, horizontalAlignment = Alignment.CenterHorizontally) {
                when (s) {
                    Step.Welcome -> Welcome()
                    Step.Name -> {
                        DeviceOrb(null, 88.dp, self = true, selfName = app.selfName, showName = true)
                        Spacer(Modifier.height(NhakoSpace.s7))
                        Title(t("onboarding.name.title"), t("onboarding.name.body"))
                        Spacer(Modifier.height(NhakoSpace.s6))
                        NTextField(app.selfName, { app.selfName = it.take(32) }, Modifier.fillMaxWidth(), placeholder = t("onboarding.name.placeholder"), align = TextAlign.Center, capitalize = true, onDone = { if (app.selfName.isNotBlank()) go(1) })
                    }
                    Step.Nearby -> PermissionStep("broadcast", t("onboarding.perm.nearby.title"), t("onboarding.perm.nearby.body"))
                    Step.Notify -> PermissionStep("bell", t("onboarding.perm.notify.title"), t("onboarding.perm.notify.body"))
                    Step.Save -> {
                        BigIcon("folder-open")
                        Spacer(Modifier.height(NhakoSpace.s7))
                        Title(t("onboarding.save.title"), t("onboarding.save.body.android"))
                        Spacer(Modifier.height(NhakoSpace.s6))
                        Row(
                            Modifier.shadow(1.dp, RoundedCornerShape(NhakoRadius.md)).clip(RoundedCornerShape(NhakoRadius.md)).background(c.surface).padding(horizontal = NhakoSpace.s5, vertical = NhakoSpace.s4),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            PhIcon("folder-open", size = 20.dp, tint = c.accentText)
                            Spacer(Modifier.width(NhakoSpace.s3))
                            BasicText("Download/Nhako Beam", style = NhakoType.body.style(c.textPrimary))
                        }
                    }
                }
            }
        }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
            if (index > 0) NIconButton("caret-left", t("settings.back"), { go(-1) }, weight = IconWeight.Bold)
            when (step) {
                Step.Nearby, Step.Notify -> {
                    NButton(t("onboarding.perm.skip"), { go(1) }, variant = ButtonVariant.Secondary, size = ButtonSize.Lg)
                    NButton(t("onboarding.perm.allow"), {
                        if (Build.VERSION.SDK_INT >= 33) {
                            permission.launch(if (step == Step.Nearby) Manifest.permission.NEARBY_WIFI_DEVICES else Manifest.permission.POST_NOTIFICATIONS)
                        } else go(1)
                    }, Modifier.weight(1f), size = ButtonSize.Lg, full = true)
                }
                else -> NButton(
                    when (step) {
                        Step.Welcome -> t("onboarding.welcome.cta")
                        Step.Save -> t("onboarding.done.cta")
                        else -> t("onboarding.name.cta")
                    },
                    { if (step == Step.Save && needsStorage) storage.launch(Manifest.permission.WRITE_EXTERNAL_STORAGE) else go(1) },
                    Modifier.weight(1f), size = ButtonSize.Lg, full = true, enabled = step != Step.Name || app.selfName.isNotBlank(),
                )
            }
        }
    }
}

@Composable
private fun Title(title: String, body: String) {
    val c = Nhako.colors
    BasicText(title, style = NhakoType.title1.style(c.textPrimary).copy(textAlign = TextAlign.Center))
    Spacer(Modifier.height(NhakoSpace.s3))
    BasicText(body, style = NhakoType.body.style(c.textSecondary).copy(textAlign = TextAlign.Center))
}

@Composable
private fun BigIcon(name: String) {
    val c = Nhako.colors
    val pop = remember { Animatable(0.5f) }
    LaunchedEffect(Unit) { pop.animateTo(1f, NhakoMotion.bouncy.spec()) }
    Box(Modifier.size(96.dp).graphicsLayer { scaleX = pop.value; scaleY = pop.value }.clip(RoundedCornerShape(28.dp)).background(c.accentSubtle), contentAlignment = Alignment.Center) {
        PhIcon(name, size = 44.dp, tint = c.accent)
    }
}

@Composable
private fun PermissionStep(icon: String, title: String, body: String) {
    BigIcon(icon)
    Spacer(Modifier.height(NhakoSpace.s7))
    Title(title, body)
}

/** Two devices and the beam between them, arriving one after another. */
@Composable
private fun Welcome() {
    val c = Nhako.colors
    val a = remember { Animatable(0.5f) }
    val beam = remember { Animatable(0f) }
    val b = remember { Animatable(0.5f) }
    LaunchedEffect(Unit) {
        a.animateTo(1f, NhakoMotion.bouncy.spec())
    }
    LaunchedEffect(Unit) {
        delay(80)
        b.animateTo(1f, NhakoMotion.bouncy.spec())
    }
    LaunchedEffect(Unit) {
        delay(140)
        beam.animateTo(1f, NhakoMotion.smooth.spec())
    }
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(76.dp).graphicsLayer { scaleX = a.value; scaleY = a.value }.shadow(8.dp, CircleShape).clip(CircleShape).background(c.surface), contentAlignment = Alignment.Center) {
            PhIcon("laptop", size = 34.dp, tint = c.accent)
        }
        Box(Modifier.width(72.dp).height(6.dp).graphicsLayer { scaleX = beam.value }.clip(CircleShape).background(c.accent))
        Box(Modifier.size(76.dp).graphicsLayer { scaleX = b.value; scaleY = b.value }.shadow(10.dp, CircleShape, spotColor = c.accent).clip(CircleShape).background(c.accent), contentAlignment = Alignment.Center) {
            PhIcon("device-mobile", size = 34.dp, tint = c.onAccent, weight = IconWeight.Fill)
        }
    }
    Spacer(Modifier.height(NhakoSpace.s8))
    Title(t("onboarding.welcome.title"), t("onboarding.welcome.body"))
    Spacer(Modifier.height(NhakoSpace.s6))
    Column(verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
        for ((icon, text) in listOf("lock-simple" to "onboarding.welcome.point.encrypted", "lightning" to "onboarding.welcome.point.fast", "share-network" to "onboarding.welcome.point.anywhere")) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                PhIcon(icon, size = 20.dp, tint = c.accent)
                Spacer(Modifier.width(NhakoSpace.s3))
                BasicText(t(text), style = NhakoType.body.style(c.textSecondary))
            }
        }
    }
}

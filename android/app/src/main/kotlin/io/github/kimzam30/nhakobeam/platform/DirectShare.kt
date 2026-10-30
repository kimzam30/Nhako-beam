package io.github.kimzam30.nhakobeam.platform

import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.content.pm.ShortcutManager
import android.os.Build
import androidx.core.content.ContextCompat
import androidx.core.content.pm.ShortcutInfoCompat
import androidx.core.content.pm.ShortcutManagerCompat
import androidx.core.graphics.drawable.IconCompat
import androidx.core.graphics.drawable.DrawableCompat
import io.github.kimzam30.nhakobeam.QuickSendActivity
import io.github.kimzam30.nhakobeam.R
import io.github.kimzam30.nhakobeam.model.Device
import io.github.kimzam30.nhakobeam.model.DeviceKind

/**
 * Your trusted devices in Android's share sheet (Direct Share): picking one sends straight to it
 * through [QuickSendActivity], without opening the app. Kept in step with the device list; a
 * device you stop trusting leaves the sheet. Android 10 and newer (older ones show the app only).
 *
 * They also show in the app icon's long-press menu, where they open the app: hiding them there
 * (setExcludedFromSurfaces) hid them from the share sheet too on Android 15, and the androidx
 * helper didn't pass them on at all, so they go straight to Android's ShortcutManager.
 */
object DirectShare {
    /** The share target category in res/xml/shortcuts.xml. */
    const val CATEGORY = "io.github.kimzam30.nhakobeam.category.SEND_TO_DEVICE"

    private var shown: List<Triple<String, String, DeviceKind>>? = null

    fun update(context: Context, devices: List<Device>) {
        if (Build.VERSION.SDK_INT < 29) return
        val max = runCatching { ShortcutManagerCompat.getMaxShortcutCountPerActivity(context) }.getOrDefault(4).coerceIn(1, 8)
        val trusted = devices.filter { it.trusted }
            .sortedWith(compareByDescending<Device> { it.reachable || it.wakeable }.thenBy { it.name })
            .take(max)
        val key = trusted.map { Triple(it.id, it.name, it.kind) }
        if (key == shown) return
        shown = key
        val shortcuts = trusted.mapIndexed { rank, d ->
            ShortcutInfoCompat.Builder(context, d.id)
                .setShortLabel(d.name)
                .setLongLived(true)
                .setRank(rank)
                .setCategories(setOf(CATEGORY))
                .setIcon(icon(context, d.kind))
                .setIntent(Intent(Intent.ACTION_DEFAULT, null, context, QuickSendActivity::class.java))
                .build()
        }
        runCatching { context.getSystemService(ShortcutManager::class.java).setDynamicShortcuts(shortcuts.map { it.toShortcutInfo() }) }.onFailure {
            shown = null
            android.util.Log.w("nhako", "Couldn't update the share sheet's devices: ${it.message}")
        }
    }

    /** The device's kind in white on the brand blue, as an adaptive icon (108 dp, art in the middle 72). */
    private fun icon(context: Context, kind: DeviceKind): IconCompat {
        val px = (108 * context.resources.displayMetrics.density).toInt()
        val bitmap = Bitmap.createBitmap(px, px, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bitmap)
        canvas.drawColor(ContextCompat.getColor(context, R.color.icon_background))
        val res = when (kind) {
            DeviceKind.Laptop -> R.drawable.ph_laptop_fill
            DeviceKind.Desktop -> R.drawable.ph_desktop_tower_fill
            DeviceKind.Phone -> R.drawable.ph_device_mobile_fill
            DeviceKind.Tablet -> R.drawable.ph_device_tablet_fill
        }
        ContextCompat.getDrawable(context, res)?.mutate()?.let { art ->
            DrawableCompat.setTint(art, Color.WHITE)
            val size = px * 40 / 108
            val at = (px - size) / 2
            art.setBounds(at, at, at + size, at + size)
            art.draw(canvas)
        }
        return IconCompat.createWithAdaptiveBitmap(bitmap)
    }
}

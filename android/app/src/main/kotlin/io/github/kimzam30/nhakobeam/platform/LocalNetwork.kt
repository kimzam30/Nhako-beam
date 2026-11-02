package io.github.kimzam30.nhakobeam.platform

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.content.ContextCompat

/** Finding devices on the same Wi-Fi. Android 13 and newer ask for Nearby Wi-Fi devices, and apps
 *  built for Android 17 also need local network access: without it the system refuses every
 *  packet to the local network (mDNS and the beacon fail with "Operation not permitted"), so only
 *  the relay works and devices are found by code alone. Both sit in the Nearby devices group, so
 *  one system prompt covers them. */
object LocalNetwork {
    /** What to ask for, newest Android first. Empty where nothing needs asking. */
    val permissions: Array<String>
        get() = when {
            Build.VERSION.SDK_INT >= 37 -> arrayOf(Manifest.permission.NEARBY_WIFI_DEVICES, Manifest.permission.ACCESS_LOCAL_NETWORK)
            Build.VERSION.SDK_INT >= 33 -> arrayOf(Manifest.permission.NEARBY_WIFI_DEVICES)
            else -> emptyArray()
        }

    /** Whether the system lets the app reach the local network. Before Android 17 it always does. */
    fun allowed(context: Context): Boolean =
        Build.VERSION.SDK_INT < 37 ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_LOCAL_NETWORK) == PackageManager.PERMISSION_GRANTED

    /** The app's page in system Settings, for when the prompt was turned down for good. */
    fun openSettings(context: Context) {
        runCatching {
            context.startActivity(
                Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.packageName, null))
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            )
        }
    }
}

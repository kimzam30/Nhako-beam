package io.github.kimzam30.nhakobeam

import android.app.Notification
import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import androidx.test.core.app.ApplicationProvider
import io.github.kimzam30.nhakobeam.platform.BootReceiver
import io.github.kimzam30.nhakobeam.platform.Notifier
import io.github.kimzam30.nhakobeam.platform.ReceiverService
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

/** Always ready to receive (Phase 20): when the receiver service runs, and what it shows. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36])
class AlwaysReadyTest {
    private val nhako = ApplicationProvider.getApplicationContext<NhakoApplication>()
    private val shadow = shadowOf(nhako)

    private fun startedReceiver(): Boolean {
        var found = false
        while (true) {
            val next = shadow.nextStartedService ?: break
            if (next.component?.className == ReceiverService::class.java.name) found = true
        }
        return found
    }

    private fun stoppedReceiver(): Boolean {
        var found = false
        while (true) {
            val next = shadow.nextStoppedService ?: break
            if (next.component?.className == ReceiverService::class.java.name) found = true
        }
        return found
    }

    private fun boot() = BootReceiver().onReceive(nhako, Intent(Intent.ACTION_BOOT_COMPLETED))

    @Before fun setUp() {
        nhako.app.onboarded = true
        nhako.app.alwaysReady = true
        startedReceiver()
        stoppedReceiver()
    }

    @Test fun startsAtBootOnceSetUp() {
        boot()
        assertTrue(startedReceiver())
        assertTrue(nhako.background.alwaysOn)
    }

    @Test fun notBeforeOnboarding() {
        nhako.app.onboarded = false
        boot()
        assertFalse(startedReceiver())
    }

    @Test fun notWhenSwitchedOffAndStopsWhenSwitchedOff() {
        boot()
        assertTrue(startedReceiver())
        nhako.app.changeAlwaysReady(false)
        assertTrue(stoppedReceiver())
        assertFalse(nhako.background.alwaysOn)
        boot()
        assertFalse(startedReceiver())
        nhako.app.changeAlwaysReady(true)
        assertTrue(startedReceiver())
    }

    @Test fun pauseLastsUntilTheAppIsOpened() {
        boot()
        startedReceiver()
        nhako.background.pauseAlways()
        assertTrue(stoppedReceiver())
        // A restart of the receiver while paused doesn't bring it back...
        nhako.background.syncAlways()
        assertFalse(startedReceiver())
        // ...opening the app does.
        nhako.background.foreground = true
        assertTrue(startedReceiver())
        nhako.background.foreground = false
    }

    @Test fun theServiceShowsAQuietNotificationWithPause() {
        boot()
        val service = Robolectric.buildService(ReceiverService::class.java).create().get()
        service.onStartCommand(null, 0, 1)
        val n: Notification = shadowOf(service).lastForegroundNotification
        assertNotNull(n)
        assertEquals(Notifier.CHANNEL_READY, n.channelId)
        assertEquals(1, n.actions.size)
        assertEquals(shadowOf(service).lastForegroundNotificationId, Notifier.ALWAYS_ID)
    }

    @Test fun aServiceStartedWhenNotWantedStopsItself() {
        nhako.app.alwaysReady = false
        val service = Robolectric.buildService(ReceiverService::class.java).create().get()
        service.onStartCommand(null, 0, 1)
        assertTrue(shadowOf(service).isStoppedBySelf)
        assertNull(shadowOf(service).lastForegroundNotification?.takeIf { !shadowOf(service).isForegroundStopped })
    }

    @Test fun manifestDeclaresAConnectedDeviceServiceAndABootReceiver() {
        val pm = nhako.packageManager
        val info = pm.getServiceInfo(ComponentName(nhako, ReceiverService::class.java), PackageManager.ComponentInfoFlags.of(0))
        assertEquals(ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE, info.foregroundServiceType)
        val boot = pm.queryBroadcastReceivers(Intent(Intent.ACTION_BOOT_COMPLETED).setPackage(nhako.packageName), PackageManager.ResolveInfoFlags.of(0))
        assertTrue(boot.any { it.activityInfo.name == BootReceiver::class.java.name })
    }
}

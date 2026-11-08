package io.github.kimzam30.nhakobeam.state

import io.github.kimzam30.nhakobeam.model.Device

// Who's in the Send to list, and in what order. The desktop has the same rules in
// src/lib/state/sendList.ts; change both together.

private val byName = compareBy<Device>({ it.name.lowercase() }, { it.id })

/** Same name, type and system: a reinstall gives a device a new key, and this is all that's left. */
private fun looksLike(a: Device, b: Device) = a.id != b.id && a.name == b.name && a.kind == b.kind && a.os == b.os

/** Without a trusted device that's offline while one just like it is online: most likely it from
 *  before a reinstall. Settings still lists it, with Remove. */
fun withoutTwins(devices: List<Device>): List<Device> {
    val online = devices.filter { it.trusted && it.reachable }
    return devices.filter { d -> !(d.trusted && !d.reachable && online.any { looksLike(it, d) }) }
}

/** By name, never by state: a row doesn't move under a finger when a device changes route or sleeps. */
fun sendGroups(devices: List<Device>): Pair<List<Device>, List<Device>> =
    withoutTwins(devices).filter { it.trusted }.sortedWith(byName) to devices.filter { !it.trusted && it.reachable }.sortedWith(byName)

/** The engine only asks a device on this network. Which path a connection uses doesn't matter. */
fun canTrust(d: Device): Boolean = !d.trusted && d.reachable && d.nearby

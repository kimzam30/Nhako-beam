package io.github.kimzam30.nhakobeam.state

import io.github.kimzam30.nhakobeam.model.Device

// Who's in the Send to list, and in what order. The desktop has the same rules in
// src/lib/state/sendList.ts; change both together.

private val byName = compareBy<Device>({ it.name.lowercase() }, { it.id })

/** By name, never by state: a row doesn't move under a finger when a device changes route or sleeps. */
fun sendGroups(devices: List<Device>): Pair<List<Device>, List<Device>> =
    devices.filter { it.trusted }.sortedWith(byName) to devices.filter { !it.trusted && it.reachable }.sortedWith(byName)

/** The engine only asks a device on this network. Which path a connection uses doesn't matter. */
fun canTrust(d: Device): Boolean = !d.trusted && d.reachable && d.nearby

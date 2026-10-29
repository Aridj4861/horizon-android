# Horizon Android 0.7.4

Changes in development after 0.7.3 (versionCode 25).

- **Vulkan / GPU batching** uses flattened terrain draws for medium and large
  views on capable Android GPUs. Storage pages respect the device's shader
  binding limit, including 128 MiB devices. Compatibility mode retains its
  ordinary drawing path. Performance still depends on the GPU and scene.
- Performance reports separate GPU preparation/uploads, world rendering and
  post-processing/hand/HUD work. Android's total GPU timer also includes in-frame
  preparation; earlier section copies are outside that interval.
- Android recognizes the application as a game for system game tools.
- Shader search has separate **Horizon** and **Modrinth** sources. Native packs
  keep the Horizon registry path; classic Modrinth ZIPs have experimental
  compatibility, checksum/archive checks, install status and Installed management.
- Native Android input for standard USB and Bluetooth gamepads: positional
  buttons, sticks, D-pad hats and analog/digital triggers use Controller Settings
  bindings, deadzones and look sensitivity. Controller focus navigation also
  works in the launcher.
- **Input device**, in Options > Controls and launcher Settings: Auto, Touch,
  Keyboard & mouse or Controller. Auto follows meaningful input; connecting an
  idle controller does not replace touch. Explicit Touch ignores gamepad gameplay
  input. The choice is saved between launches.
- **Controls in inventory**, in Touch Controls, optionally keeps the overlay
  visible. Custom keys and utility buttons remain available; Inventory/Pause
  close the inventory. Movement/combat buttons do not act on inventory slots or
  move the player behind the screen. Slot taps still work directly.
- **Toggle sprint button**, in Touch Controls: tap on/off instead of hold, with
  visible pressed state. Clearing touch input or losing focus resets the latch.
  Both new touch settings default to off.
- Merge 233 upstream commits across two reviewed batches, including merge history,
  through `13d7d9995`:
  asynchronous single-player startup, connection screens, faster disconnects,
  team-aware multiplayer collisions, packet validation, item component handling,
  native-mod hooks and gameplay fixes. The later batch adds render-crate separation,
  native item/recipe panels, sprite decoding caches, combat/projectile fixes,
  world-save recovery and structure/LOD corrections. Android sprite caches use
  the clearable system cache directory. Upstream deployment workflows stay disabled.

## Checks before publishing

Host input tests: `python tools/test_android_input.py`.
Shader catalog/install tests: `python tools/test_android_shaders.py`; add `--live`
to check Modrinth metadata without downloading shader packs.
GPU arena, upload, shader and timestamp tests: `python tools/test_android_gpu.py`
(requires a host Vulkan/DX12 GPU with encoder timestamps; does not open a game).

Device testing still needs to cover:

- USB and Bluetooth: movement, camera, triggers, bumpers, quick taps, menu
  accept/back, Start, D-pad diagonals, disconnect/reconnect and screen off/on.
- Auto switching and explicit Touch with a controller connected. Verify the
  in-game choice also appears in launcher Settings after returning to it.
- Inventory overlay on/off: slot taps, custom keys, closing the inventory and
  utility buttons. No world movement or accidental item dropping from controls.
- Sprint hold/toggle: second tap stops sprinting; check pause, inventory and sleep.

Controller firmware must expose an Android gamepad. A keyboard/media mode that
sends system volume keys must be switched on the controller itself; those keys
are not repurposed. External-controller rumble is not implemented; the existing
Vibration setting still controls the phone vibrator.

The intermittent low-FPS GPU plateau remains under investigation. Timing reports
do not establish its root cause. No measured phone FPS improvement is claimed.

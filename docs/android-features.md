# Android feature audit (0.6.6)

This is an implementation audit, not a claim that every Horizon menu has been
tested on every phone. Physical-device performance has not been independently measured for 0.6.6.
Emulator frame counters must not be advertised as phone performance.

| Horizon feature | Android integration / remaining work |
|---|---|
| Controller action mapping | Touch uses the same actions, default Java eight-direction or optional analog movement, and toggle sneak. Hardware controller enumeration/stick input still needs a native Android backend; do not infer it from desktop gilrs support |
| Touch layouts | Four explicit schemes, normal Options > Controls settings, 16–64 px circle, optional 360-degree movement, sprint visibility, editable layouts, scale/opacity sliders and handedness |
| Contextual touch | Use action for held items/entities; movement actions serve flight/swimming. Not all Bedrock contextual icons/gestures are implemented |
| Controller vibration settings | Phone `Vibrator` now wired to existing game events and the existing vibration switch; no adaptive triggers or external-pad vibration backend |
| Unicode keyboard | GameActivity IME and composition; system keyboard remains responsible for its own clipboard suggestions |
| Horizon clipboard actions | Newly connected to Android ClipboardManager for explicit text copy/paste; no background clipboard polling or file-URI reads |
| Sound and subtitles | AAudio mixer and downloaded sound assets; existing category volumes, subtitles and accessibility settings remain available |
| Graphics / streaming | Existing game resolution, distance, FPS, quality and worker controls. Launcher selects real Vulkan draw paths and does not override them |
| Native mods | Wasmtime `.hmod` runtime; existing capabilities and errors appear in the game mod menu. Direct HTTPS import; no Java-mod search or Java/Mixin execution |
| Resource packs | Checksum-verified resource-pack search/install; modpack browsing, importing, selection and automatic launch hooks are removed on Android |
| Single player | Default integrated server and optional in-frame path. Editor pauses unpublished single player through the engine pause decision |
| Servers | Existing protocol 777 client. Manual matching-version server address supported; Android device-code sign-in and session handoff implemented; temporary Switch dev ID with Microsoft Live device-code flow, browser Custom Tab, code copy and persistent pending login; initial code request accepted, full account login unverified |
| Play with Friends | Shared core exists, but relay/account/clipboard/background behavior needs an Android end-to-end pass before advertising parity |
| Saves and backups | Engine save logic works; Android Storage Access Framework import/export and share UI remain to implement. Uninstalling the app deletes internal data |
| Lifecycle | Touch/key cancellation, vibration stop and surface release/recreation. Leaving the game returns to the native launcher; background process termination still needs broader save/recovery testing |
| Android file access | Downloads use internal app storage; no broad storage permission. System document picker and URI-permission handling remain absent |
| Physical device tuning | Thermal/frame-pacing and GPU-driver comparisons need measurements on the target phone; an emulator is not evidence of phone performance |
| Desktop updates / telemetry services | Desktop deployment/auto-update infrastructure is not an Android updater; fork exports keep original workflows inactive |

Priorities after this change: world import/export, Minecraft Services approval and production account validation,
hardware input devices, then measured lifecycle and thermal behavior. Preserve
the current engine performance baseline while adding these platform bridges.

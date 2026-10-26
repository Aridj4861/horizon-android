# Android development port

Horizon runs its Rust simulation and wgpu Vulkan renderer in Android GameActivity.
This build implements **Minecraft 26.3**. It is not a launcher for arbitrary Java
versions.

## Install and play

Install the ARM64 APK on an Android phone (Android 9+, Vulkan), or the x86_64 APK
in an x86 Android emulator. Open the app and tap **Download 26.3**. The app fetches
Minecraft's version metadata, client resources and supplemental assets directly
from Mojang over HTTPS, verifies SHA-1 and file sizes, then installs atomically.
The first download is approximately 501 MiB, including official sound assets.
Tap **Play** after installation. Subsequent launches use installed resources
offline. No computer, local server, ADB forward, or phone Java runtime is needed.
Use your own licensed Minecraft copy.

The APK contains the native engine and version-specific generated engine tables.
Game textures, models, fonts, languages and menu backgrounds are downloaded from
Mojang. The engine tables are prepared by the developer during APK creation,
not by the player. Updating the implemented Minecraft version requires an engine
update, not just choosing another client JAR.

Saves and download caches live in the app's internal data directory. Download
failures retain verified cache entries for retry; partial files are never treated
as installed resources. Uninstalling removes app data.

## Touch controls

Version 0.5.0 implements the three interaction schemes named in the official
[Bedrock changelog](https://www.minecraft.net/en-us/article/minecraft-1-21-60-bedrock-changelog):

- **Joystick & aim crosshair**: eight-direction movement (optional 360-degree analog movement), camera drag, attack/mine and
  use/place buttons, jump, sneak and sprint.
- **Joystick & tap to interact**: tap a world point to use/place, tap an entity
  to attack, hold to mine. Held consumables, shields and ranged items use the
  held-use action instead. Dragging turns the camera and cancels a world tap.
- **D-Pad & tap to interact**: directional movement with the same world gestures.
- **D-Pad & aim crosshair**: directional movement with crosshair aiming and action buttons.

The top-center row contains F3, chat, pause, Options and F5 camera perspective.
Open **Options > Controls > Touch Controls** to choose a scheme, adjust touch
sensitivity and circle size (16–64 GUI pixels, default 32), enable 360-degree
movement, change handedness, or show/hide the Sprint button. The settings use
the same buttons, sliders, scrolling and Back/Done navigation as other options.
**Auto Sprint** is in Controls next to **Auto-Jump**. Eight-direction Java input
remains the default; the optional analog path changes prediction direction while
the Java protocol's directional key flags remain boolean.
**Customize layout** opens the draggable HUD editor with size/opacity sliders.
In D-pad mode the center button toggles sneak; Swap jump/sneak exchanges it
with the jump button. Zero opacity hides a control while the editor still
shows a faint preview so it can be recovered.
Each scheme keeps an independent layout. Done saves `horizon-touch.json` in the
current profile; Cancel/Android Back restores the previous layout, and Reset
restores the current scheme's positions, sizes and opacity. Single-player
pauses during editing; a remote server continues running. No game actions are
sent by editor touches. Switching off controller support does not disable touch.

The nine hotbar slots select items; the movable ellipsis opens inventory.
The optional use button appears for eligible targets in tap modes when enabled. These
controls map to existing Horizon actions, including flight/swimming movement,
controller rebinding and toggle sneak. They do not replace vanilla gameplay.
Version 0.5.1 replaces the drawn icons with original normal/pressed PNGs from
[Mojang bedrock-samples](https://github.com/Mojang/bedrock-samples/tree/46ba6ea985fb5a92d79a9419198f10dda14c199d/resource_pack/textures).
Flight, swimming, dismount and diagonal D-pad states use their original sprites.
Texture bytes remain private in `.local/`, with original licensing/provenance.
This is an adaptation of Bedrock's schemes, not verified pixel-for-pixel parity:
emotes, every mount-specific icon, inventory stack-splitting gestures and all
Bedrock touch options are not implemented. Projectile launch direction still
follows the engine's player view; use crosshair mode for precise ranged aiming.

Menus support tapping, list swipes, slider dragging and inventory dragging.
The **Back** button closes inventory/screens; Android Back closes the native
keyboard first. The compact **...** drawer retains F1-F12, F5 perspective,
chat, the native keyboard, swap/drop and other Java keys. Hold different keys
with separate fingers, or latch **HoldF3** before another F-key. F3+F6 remains
an upstream engine stub; the key is delivered but no Debug Options screen exists.

## Android platform features

The existing controller **Vibration** switch now controls the phone's vibrator.
Hurt, block destruction, explosions and totem events use Horizon's existing
pulse definitions through Android's [VibrationEffect](https://developer.android.com/reference/android/os/VibrationEffect).
No vibrator is a supported case. Vibration stops on suspend/focus loss and when
the option is disabled. Actual feel and amplitude need phone testing; emulator
startup cannot establish physical vibration quality. External-controller rumble
and DualSense adaptive triggers are not supplied by the phone-vibrator backend.

Horizon's copy/paste now uses the Android [system clipboard](https://developer.android.com/develop/ui/views/touch-and-input/copy-paste)
for explicit text-edit shortcuts and copy actions. Text-only clipboard access
avoids resolving file/content URIs. Gboard's normal paste UI also remains usable.
AAudio, native Unicode IME, lifecycle surface recreation, native `.hmod` mods,
resource packs and the engine's graphics settings are compiled in. See the
[Android feature audit](android-features.md) for remaining platform gaps.

## Mobile launcher

The native launcher now has Play, Mods, Packs, Modpacks, Installed and Settings
pages. The mobile UI shares the desktop launcher's actual `launcher/src/browser`
backend: Modrinth search, version selection, dependency resolution, checksum
verification, Java content compatibility checks, install and remove.

- **Mods** searches Java content mods. Their data/assets may port; their Java,
  Fabric/NeoForge mixins and behavior do not run. Installation reports unsupported
  projects instead of treating a downloaded JAR as a running mod.
- To import a native Horizon mod, paste its direct **HTTPS `.hmod` URL** into the
  Mods search field and press Search/Done. HTTPS is enforced through redirects,
  the download is capped at 32 MiB, and its manifest/package is validated. New
  native mods remain disabled until enabled in Installed. Only use sources you
  trust; these direct downloads are not an authenticated Horizon mod index.
- **Packs** downloads resource packs. Installed toggles them in the profile's
  options; removing a pack also removes its enabled entry.
- **Modpacks** downloads and runs the existing Rust import/inventory/port/report
  pipeline. Choose an imported pack with Play's Modpack profile selector. Only
  non-conflicting carried content is loaded. A pack with no loadable content is
  rejected. This does not make arbitrary Java modpacks compatible.
- **Installed** manages browser additions and native `.hmod` enablement. The
  game's Options > Mods shows each mod's capabilities and runtime errors.
- **Settings** selects a rendering mode, profile, offline player name and server.
  Rendering modes and profiles use explicit selection lists. Modpacks have a
  separate selection list; installed additions have switches showing their state.
  Profiles can be created without the old five-profile limit. Profile zero keeps
  existing Android saves. Name and server fields use the system keyboard.
- Graphics options (resolution, chunk distance, FPS, quality and worker budgets)
  belong to the game's settings. The launcher does not rewrite `options.txt` or
  inject a render-distance argument. Old launcher graphics fields are ignored on
  upgrade; existing game settings are preserved until the player changes them.

This is an Android UI and a port of the content/launch functionality, not full
desktop-launcher parity. Desktop binary/self-update installation, OS keychain
account integration, desktop agent tools and native Android file-picker import
are not implemented. APK upgrades are manual. Java touchscreen mods such as
[MidnightControls](https://github.com/TeamMidnightDust/MidnightControls) require
a source port to Horizon's Rust input layer; recompiling their JAR is insufficient.
The auxiliary-key design was checked against
[Pojav's controls](https://github.com/PojavLauncherTeam/PojavLauncher/blob/v3_openjdk/app_pojavlauncher/src/main/java/net/kdt/pojavlaunch/customcontrols/CustomControls.java).

## Android terrain compatibility

By default Android uses direct per-section terrain draws and disables GPU occlusion
culling, indirect first-instance/multi-draw and mappable primary buffers. This
avoids desktop-oriented paths implicated when entities render but terrain does
not. It is a compatibility mitigation for phone Vulkan drivers, not a confirmed
diagnosis for every device. The terrain path is tested on the emulator; the
reported POCO M8 Pro symptom still needs verification on that physical device.
The desktop renderer keeps its existing feature selection. Both modes use the GPU.
The rendering selection names both actual paths: **Vulkan / GPU batching**
(adapter-supported indirect draws and GPU culling) and **Vulkan / compatibility**
(direct terrain draws). These use the same system Vulkan driver and API. There
are no bundled alternative OpenGL, ANGLE or custom Turnip drivers in this build.

## Performance and sound

APKs use the optimized Rust `fast` profile. Graphics limits and presets are
controlled by the game itself, including the existing resolution slider. The
launcher no longer applies the old 50% / 60 FPS / four-chunk mobile preset.
Changes made by that earlier preset stay as ordinary game settings; resetting
those automatically would overwrite player choices.

The F3 overlay has its own vanilla `debugGuiScale` option in the profile's
`options.txt`: `0` uses the automatic debug scale, `-1` follows the GUI scale,
and positive values request an explicit scale (bounded by the GUI scale).
Horizon currently has no Debug Options screen for changing this visually.

Audio is compiled in and the installer downloads the official sound assets with
checksum verification. An upgrade from an older silent APK prompts for resource
installation again; existing cached objects are reused and saves stay in place.
Android GameActivity supplies the system IME, including Unicode and composition;
no custom Android keyboard or additional keyboard/audio permission is required.

## Build (developers only)

Install Rust, Python 3.11+, Android SDK (platform 35+, build tools), NDK, and a JDK
for APK signing, and Gradle 8.14. Android Studio's bundled JDK works.
GameActivity 4.4.0 and AppCompat are resolved from Google Maven by Gradle. Prepare the pinned engine
tables once on a development computer, then build:

```sh
rustup target add aarch64-linux-android x86_64-linux-android
python tools/android_prepare.py
python tools/android_bedrock_controls.py
python tools/android_build.py --abi arm64-v8a
python tools/android_build.py --abi x86_64
# Distribution APK: original resources download on the phone.
python tools/android_build.py --abi arm64-v8a --public
```

Windows discovers `%LOCALAPPDATA%/Android/Sdk` and Android Studio's JDK. Elsewhere
pass `--sdk PATH --java-home PATH` or set `ANDROID_HOME` / `JAVA_HOME`. APKs go to
`.local/android/apk/<abi>/horizon-android.apk`, signed with the development key in
`.local/android/debug.keystore`. The default `--profile fast` is optimized;
`--profile dev` is for debugging. `--gradle PATH` overrides cached Gradle discovery.
The build packages generated tables into
`assets/engine-data.zip`, excluding game assets, Java classes and game JARs.
Generated tables and APKs stay in `.local/`; never commit them. Publish only the
audited `horizon-android-public.apk` produced by `--public`, not developer APKs.
No `bootstrap.json` or local download URL is required or embedded.

Default developer APKs are debuggable; `--public` APKs are not. Cleartext traffic
is disabled, the resource downloader
accepts only official Mojang HTTPS hosts and does not follow redirects. Every
client and asset file is checksum-verified before use.

## Check

```sh
python tools/android_build.py --abi x86_64 --check
python tools/android_build.py --abi x86_64 --clippy
cargo test -p horizon-client --lib --no-default-features touch
cargo test -p horizon-client --lib --no-default-features ime_edit::tests
cargo test -p horizon-client --lib --no-default-features official_install::tests
python tools/android_ctl.py --serial SERIAL state
```

The control helper reads the debug app's private descriptor through `run-as`,
forwards one loopback port for one command, then removes it. It never prints the
authentication token. This forward is only for development control, not downloads.
For in-frame smoke tests, place an argument array such as
`["--in-frame", "--world", "Android In Frame", "--creative"]` in internal
`files/android-launch.json`, restart and tap Play. Remove it afterwards to restore
the default server-thread mode.

Use fresh AVDs and fresh game/launcher/XDG folders for agent tests, with
`HORIZON_LAUNCHER_NO_SELF_UPDATE=1` and `HORIZON_NO_AUDIO=1`. Never test on existing
player data. An emulator's host GPU is not evidence of physical phone performance.

## Android 0.6 update

The Android branch merges upstream through `6a065fa4` (286 incoming commits),
including world-generation noise, mesh lookup/culling, chunk batching, networking,
save handling, and resource-pack fixes. Upstream shader-pack and Discord presence
features retain their upstream defaults; the Android compatibility Vulkan mode
remains available. No forced render-distance, FPS or resolution limits are added.
Phone performance still needs measurement on the physical GPU.

Leaving the native game returns to the native launcher in the same app process.
Predicted block destruction on the integrated-server/multiplayer client path now
requests the same vibration as the in-frame path; enable controller vibration.
Classic D-pad crouching uses a double tap, while fly/swim descent is held.
The touch editor supports deselecting a control and applying size/opacity to all
controls in the current mode. It uses original privately prepared Bedrock sprites;
pixel-perfect positioning for every Bedrock version/device is not yet verified.

The `.hmod` page imports native packages from a direct HTTPS URL and no longer
searches Java mods. Fabric/Forge/JAR code does not execute in this Rust engine.
Resource packs remain supported. Android 0.6.1 removes the modpack UI and launch pipeline.

### Microsoft / Xbox account on Android (dev build)

Android 0.6.2 temporarily uses the public Minecraft Nintendo Switch application
identifier `00000000441cc96b`, as requested by the fork owner for development.
The identifier is listed in [PrismarineJS's public title constants](https://github.com/PrismarineJS/prismarine-auth/blob/master/src/common/Titles.js).
It is not Horizon's own application registration and this does not mean Mojang
has approved this fork. The previous owner's client ID is no longer bundled or
read from saved launcher configuration. Existing account tokens are not bundled.

This registration requires Microsoft Live's `oauth20_connect.srf` and
`oauth20_token.srf`, with the Xbox Live service scope, rather than Entra's device
endpoint. Login uses the existing Xbox/XSTS, Java entitlement and profile checks;
there is no license or server authentication bypass. The initial Live device-code
request was accepted; a complete sign-in needs account testing.

Sign in opens a browser Custom Tab automatically. The code is copied to Android's
clipboard and passed to Microsoft's `link?otc=...` page. If the browser does not
prefill it, paste the copied code. The code is also a tappable Copy button in the
account panel. Back hides the panel and keeps the request pending; Cancel sign-in
explicitly cancels it. Browser suspension does not cancel the worker. Since 0.6.5,
refresh/access tokens are encrypted with an app-owned Android Keystore AES-GCM key
and saved in app-private storage. The game still receives a private one-use file.

For a future production build, use the project's own approved registration.
The [Minecraft App ID review form](https://aka.ms/mce-reviewappid) is the approval
entry point; the development identifier does not grant approval to our app.

The comparison port `CornyBirdy/horizon4android` was reviewed as a separate WIP:
its documentation marks physical-device runtime/input checks as outstanding.
Its Java activity implementation was not imported into this native Rust launcher.

## Android 0.6.1

Version 0.6.1 configured the owner's public registration. Version 0.6.2 replaces
that configuration with the fixed temporary dev identifier described above.

Removed the Modpacks tab, modpack picker on Play, importing/conversion actions
and automatic `--dev-mod` launch arguments. Old `pack` selections in
`android-launcher.json` are ignored; launching uses the selected normal profile.
Old imported directories, worlds and their manifest records are not deleted.
They remain in `profiles/profile-N/ported-packs/` (or `ported-packs/` for the
default profile) for manual recovery. Installed shows native `.hmod` packages
and resource packs. The desktop launcher's modpack support remains upstream's.

## Android 0.6.2

The temporary dev registration uses Microsoft Live device-code authentication,
with its distinct endpoint, scope and Xbox ticket prefix. Live session cookies
are retained only while a device-code request is pending. Microsoft accepted the
initial request; a complete licensed-account sign-in has not been tested here.
The browser uses the documented Custom Tabs intent protocol, with an ordinary
browser fallback. Closing the account panel no longer cancels an active request.

## Android 0.6.3: system keyboard

Keep the native IME open for title-menu text editors: the title also uses the
engine's `Playing` screen tag, so the world-only dismissal must check that no
title is open. Surface recreation no longer ends the text session. The in-world
keyboard shortcut opens chat before displaying the IME.

On the isolated API 35 AVD, the previous build closed the IME on Add Server's
address field. The fixed build kept it open, accepted an address, reopened on
another field and saved the address to the test profile's server list.


## Android 0.6.4: touch gestures and controls

On Android, Options > Controls > Touch Controls opens the same settings as the
gear beside Pause. Select a control scheme on the settings page, then Customize
layout to move buttons and change size/opacity. Settings use visible switches;
the sensitivity slider accepts dragging. Save/Done persists the profile layout;
Cancel or Android Back restores the settings from before the editor opened.
The editor toolbar leaves the default chat, pause and inventory buttons exposed.

In tap mode a short tap on a mob attacks; holding a mob performs an available
interaction, such as feeding or trading. Holding elsewhere mines, or uses a held
consumable/shield/bow as appropriate. Once a use gesture commits, drifting the
same finger cannot turn it into camera movement or mining, even if its last item
was consumed. A second finger can still turn the camera. Release ends the hold.

Non-crosshair controls draw a touch circle and the current mining progress at
the finger target. The centre block outline is hidden when there is no active
touch target. Crosshair mode keeps its ordinary centred target.
The factory sprint button sits in the right action cluster near Jump, following
Minecraft's official touch-control illustrations. Only the former factory
position is migrated; customized positions are retained. Reference:
https://www.minecraft.net/en-us/article/minecraft-controls . This is a native
Horizon implementation; remaining Bedrock differences should be reported.


Validation used 15 touch-input unit tests and Android Clippy with warnings
denied. On the isolated API 35 AVD, an off-centre short tap with wheat attacked
a cow (health 10 to 9); an off-centre hold fed a fresh cow (wheat 16 to 15,
health stayed 10); a hold opened the farmer's merchant menu. A drifting hold ate
an apple (food 6 to 10), kept yaw unchanged and released item use on finger-up.
The Controls entry saved a moved sprint button and returned to Controls;
Android Back cancelled an edit. The system-keyboard fix from 0.6.3 is included.


## Android 0.6.5: persistent accounts and continuous touch input

Accounts use an Android Keystore AES-256-GCM key. The app-private files contain
ciphertext, a random 96-bit IV and an authenticated tag; the account UUID is AAD.
Writes replace a synced temporary file atomically. Sign-out deletes the account's
ciphertext. Tokens are never written to accounts.json or logs. The shared account
flow refreshes sessions on launch. A cached status chip distinguishes Signed in,
Sign in required, Signing in and Offline; returning from a game refreshes it.
Older builds kept secrets only in process memory, so updating from 0.6.4 requires
one new sign-in. Subsequent process restarts keep the account. Reinstalling/clearing
app data or restoring ciphertext on another device requires a new sign-in.

A camera swipe with food does not begin item use. A stationary 350 ms hold commits
mining or use; moving that finger afterwards turns the camera while keeping the
hold active until finger-up. The cursor follows the finger through camera motion.
Unlike 0.6.4, committed eating no longer locks camera movement. Short mob taps
still attack. Changing a mining target starts that target's own break progress.

Four explicit choices expose joystick and D-pad, each with tap or crosshair aiming.
The D-pad choices retain Bedrock's shared layout and split-controls storage.
Crosshair modes always show Use/Place. In tap modes, the extra Interact button is
**off by default** and can be enabled in Touch Controls. When enabled it appears
for a usable target, not merely because food is held. A pressed button stays
visible until its finger releases. Block hints use the engine's resolved behaviour
classes; mob hints use the existing vanilla interaction predictor.

Icons keep their source aspect ratio. Visible placement and hit testing share
bounds clamped to the drawable surface, including enlarged buttons. The gear
continues to open Touch Controls, also accessible in Options > Controls. The
launcher, startup touch editor and in-game touch editor use Minecraft's
`ui.button.click` through the engine Audio implementation and respect game volume.

Android joystick intent is mapped to eight keyboard directions before both local
prediction and the directional input packet. Deadzones still apply; 45-degree
sectors prevent small finger drift from adding a strafe key. Direction normalization
and subsequent item-use/sneak multipliers use the existing Java arithmetic. This
fixes the previous disagreement between arbitrary analogue movement and boolean
server input. It does not establish compatibility with a private server anticheat.

Validation: 18 gesture/layout tests and 16 gamepad tests; 14,960 real Java
KeyboardInput/LocalPlayer input comparisons; 424,684 evolving native sprint-scene
states; 648,760 sprint-phase output lines; and 942 movement-packet/teleport cases
(24,707 output lines) matched the pinned Java 26.3 oracle. Scope and exclusions are
recorded in each harness's report. Hypixel/Watchdog was not tested with an account.
The Keystore development probe passed encrypted roundtrip, tamper rejection,
account-AAD rejection, persistence over two app processes, and deletion. The probe
is compiled only with debug assertions and is absent from optimized APKs.

On the isolated API 35 AVD, a swipe with food only turned the view; a committed
hold ate an apple while yaw changed by 16.875 degrees and stopped use on release.
A continuous mining drag turned 39.375 degrees and broke four wall columns.
The optional button was disabled initially, enabled explicitly to open a merchant,
and did not interact when looking away. The fourth scheme persisted correctly.

Visual reference: stable Bedrock 26.52 as of 2026-10-25
([official releases](https://feedback.minecraft.net/hc/en-us/sections/360001186971-Release-Changelogs)).
The latest official sample asset set is still 1.26.50.4 at the pinned revision.
Its HUD uses a built-in 16x16 `cursor_renderer`, without a standalone circle PNG.
The cursor here is a native ring with a dark surround and an expanding inner mining circle;
bit-for-bit visual equivalence with Bedrock's closed renderer is not established.


The final optimized AVD build kept straight movement under small sideways stick
jitter (zero measured strafe). A warmed client over one persistent control
connection advanced 242 ticks in 12.093 seconds, approximately 20.01 ticks/s.
Startup catch-up and per-command ADB overhead are excluded from this timing.
No renderer/shader/chunk-streaming optimization or resolution-scale defaults
were changed by 0.6.4/0.6.5; the reported POCO FPS regression has not been measured
on the physical device. UI/input overhead remains a separate profiling concern.


## Android 0.6.6

Upstream is merged through `7fd1f311` (138 additional commits): independent
server sprint state, transfer/cookie handling, remote potion contents, container
range/crafting fixes, passenger poses, skins, particles and scheduled chunk ticks.
The new registry modules retain explicit file paths for Android's shared launcher
imports. Upstream deployment workflows remain inactive in this fork.

Remote player teams now use profile names, matching Player.getScoreboardName,
while non-player entities use UUIDs. Profile names remain available after a server
hides an NPC from the tab list. This restores collisionRule handling for those
players; ordinary entity and boat collisions still run. The team-rules Java oracle
also checks conflicting profile-name/UUID memberships. Hypixel/Watchdog requires
physical account testing; no blanket anticheat compatibility is claimed.

Old potion state and cached effect modifiers are discarded at session/world
boundaries after saving. Joining another world restores that world's own saved
effects. The Android Sprint button sends a momentary held request independently
of keyboard Toggle Sprint. Releasing movement still ends the vanilla sprint;
hiding Sprint leaves double-forward and the optional Auto Sprint available.

Pause and Settings reuse Chat's square Bedrock frame. Chat, Pause, Settings,
inventory opening, slot presses and creative-tab presses call the existing audio
engine and respect its category/master volumes. The touch circle is twice the
previous default diameter and can be resized with the normal slider.

The 0.6.5 validation above describes that release, not a fresh run of its entire
suite. Current 0.6.6 validation: 19 touch tests, 29 options tests (one existing
ignored test), one profile-team regression test and 16 gamepad tests passed.
The isolated API 35 AVD verified a sprint request with keyboard Toggle Sprint on,
stop followed by walking without sprint, Auto Sprint beside Auto-Jump, saved
52-pixel circle/analog movement/hidden Sprint settings, and layout persistence
after another option changed. A 60:100 joystick gesture moved at a measured
0.59481 lateral/forward ratio. An infinite speed effect was absent in a second
world and restored on rejoining its own saved world. Sound feedback and physical
vibration still need phone confirmation. No physical POCO FPS measurement is
claimed.


For publication, a separate `--release-candidate` packaging option disables
Android debuggability and remaps personal compiler paths before the privacy audit.
This remains a local candidate: bundled Mojang resource files prevent treating a
privacy pass as permission to publish that older APK. The later 0.6.7 distribution
mode replaces this packaging; see [publication notes](android-publication.md#publishing-the-android-application).
The final optimized emulator APK also passed launch and a background/cancel test:
partly edited HUD positions were discarded on suspend and were not written by a
subsequent options change. The team-rules oracle matched all 6000 Java cases,
including conflicting UUID/name memberships, with zero oracle errors.


The same cross-world effect isolation, restoration of the world's own saved
effect, and sprint-stop-walk sequence passed in `--in-frame` mode as well as the
default integrated-server path. Seven crash-format tests passed, including
Windows path redaction without embedding the builder's home directory.
The final local candidate's privacy audit inspected 2018 archive entries, found
no flagged private files/credential patterns/personal Windows paths, and verified
`debuggable=false`. This is a bounded check, not a guarantee against every secret
format, and it does not authorize distributing the bundled Mojang resources.
Screenshots and emulator FPS claims are intentionally excluded from the public
showcase pending the owner's verification.


## Android 0.6.7

Two editable top-row shortcuts open F3 diagnostics (the robot button) and cycle
F5 camera perspective (the people button). Both use the original regions of the
same Bedrock GUI sheet as Chat. They consume only their own touch and leave a
held movement finger active. Existing ten-control saved layouts keep their
custom positions, sizes and opacity when the two buttons are added.

The mining indicator now grows a complete inner circle towards the fixed outer
rim, rather than drawing an angular progress arc. Button UVs stay within their
own source texels and screen edges snap to physical pixels to avoid atlas-edge
sampling and fractional-pixel seams. Physical-device confirmation is still
needed for the previously reported cut edges.

Reviewed and merged 59 upstream commits through `74768d2e`, preserving the
Android workspace member and adding upstream's experimental mod UI guest. The
new OpenGL support does not replace either Android Vulkan selection.

Validation: Android x86_64 Clippy passed, all 22 touch tests passed, the upstream
item-component wire roundtrip test passed, and all nine Android source-export
and APK-audit tests passed. The optimized ARM64 0.6.7 APK compiled, signed and
passed its privacy/manifest audit. This change was not launched in the emulator;
phone visuals and gameplay remain for the owner to verify.

### 0.6.7 distribution update

The top row again uses its original spacing. Pause and Settings now preserve
the full four-pixel lower bevel of the Chat frame. Options > Controls > Touch
Controls includes **F3 text scale** (Auto or 1–6); explicit Android values can
increase the debug text independently of the HUD scale.

The mining circle uses cached unit-circle geometry and one quad per ring
segment instead of overlapping pixel squares. No physical-device FPS gain is
claimed; the reported regression still needs comparable phone measurements.
Neither render distance, resolution nor the selected Vulkan mode is overridden.

Use `python tools/android_build.py --abi arm64-v8a --public` for the distributable
APK. Original resources download on the phone; the generated engine-table bundle
is filtered by an explicit allowlist. See android-publication.md for the payload
and update behavior. VersionName remains 0.6.7 (versionCode 16).

Distribution validation: public-mode Android Clippy and the optimized ARM64 build
passed. The signed APK reports versionName 0.6.7, versionCode 16, ARM64 and no
Android debuggable flag. The public payload audit found no known secrets,
private paths or player data, and confirmed only the 32 allowlisted tables.
22 touch tests, four debug-overlay tests, the touch-options test and a synthetic
resource reconstruction test passed. A live fixture rebuilt all 19 worldgen
maps and matched 1511 structure templates against the owner's prepared copy;
all 60 pinned control/license files downloaded and passed their hashes. Seven
APK-audit tests and three source-export tests passed. No emulator or phone game
session was run for this update; gameplay and physical-device FPS need owner
confirmation.


## Android 0.6.8 (versionCode 17)

In tap modes, the native Java attack-recovery gauge moves to the hotbar when
the crosshair is absent. It uses the current weapon's actual attack strength;
the normal Attack Indicator Off setting remains respected.

Options > Accessibility has independent horizontal and vertical HUD margins,
0-12 percent. Zero preserves the existing layout. Only passive information is
laid out inside these bounds: F3, the performance display, scoreboard, boss bars,
effect icons, subtitles, titles and notifications. The full-screen world, hotbar,
inventory, menus, touch controls and all input coordinates remain unchanged.
The original HUD shader and touch-input adapter are unchanged. The layout helper
restores the full HUD dimensions before controls or interactive screens draw.

The Play page now distinguishes version checks, verified resource/sound download,
unpacking, world data preparation, touch-control downloads and finalization.
A progress bar, MiB/file counters and retry state replace the single ambiguous
download counter. Smaller launcher surfaces show a compact current-stage view.

Editable local copies of the original control PNGs are prepared at
`.local/android/editable-controls`, including `EDITING.txt`. Chat/F3/F5 are regions
of `gui.png`; the standalone UI/state icons are in `ui/` and `controls/`.
These copies do not automatically override pinned public downloads. Texture
changes can be supplied by a resource pack under
`assets/minecraft/textures/gui/bedrock/<relative path>`.

Upstream was reviewed again through `6df37324`, including the main-branch ambient
mob/block effects fix. A promotional outline and copy are in android-launch.md.
The already published 0.6.7 release artifacts remain untouched; 0.6.8 is prepared
separately for owner testing and manual publication.


Validation: Android Clippy, optimized ARM64 compilation/signing and the public
APK audit passed. Sixteen HUD tests (including shader validation and restoration
of the main viewport around passive overlays), 23 touch tests, the touch-options
test and ten Python source/audit tests passed. Source comparison confirms the
window/touch handler, controller input adapter and full-screen HUD shader are
unchanged from the published baseline. The APK is 0.6.8, versionCode 17. This
update was not launched in an emulator or on the physical phone; no new FPS
or full-gameplay result is claimed.


## Android 0.6.9: keypad, portrait and experimental legacy builds

Launch settings now offer Landscape, Portrait, reversed Landscape/Portrait and
sensor rotation. The choice is saved for the launcher and game. It does not
change the HUD safe-margin contract: those margins only inset passive information
such as F3. They do not resize the world, inventory or touch hit targets.

The native launcher's portrait layout uses phone density, stacked settings and
shorter tab labels. Short landscape surfaces scale the launcher down to keep its
settings reachable. Arrow keys move a visible focus border; Enter activates the
focused item. This also provides initial TV remote navigation. Android TV gets a
Leanback launcher entry and an original vector banner. TV support is experimental;
analog gamepad axes and full remote-only gameplay have not been validated.

`Launch settings > Physical keypad` enables the optional feature-phone preset:

| Physical key | Action |
| --- | --- |
| D-pad arrows | Walk (diagonal input normalized) |
| 2 / 8 / 4 / 6 | Look up / down / left / right |
| 1 / 3 | Attack / use |
| 5 or Enter / D-pad centre | Jump |
| 0 | Sneak |
| 7 | Inventory |
| 9 | Next hotbar slot |

This uses the existing controller movement and action pipeline. Release events,
opening a menu and losing focus clear held keys. The preset is off by default,
so ordinary keyboards retain their normal Minecraft bindings. Platform key-code
behaviour still needs a test on the physical F-03L.

In `Options > Controls > Touch Controls`, the attack indicator can sit left or
right of the hotbar. It uses actual Java attack strength and the game's sprites;
it moves farther out if an offhand item occupies that side. The normal Attack
Indicator option still controls Off/Crosshair/Hotbar. The touch layout editor has
an **Add buttons...** palette for the twelve available controls and **Remove
button** for the selection. Visibility is saved; Cancel restores it with the rest
of the layout. This palette does not yet create arbitrary key macros or duplicate
buttons.

### Experimental ARMv7

Build with `python tools/android_build.py --abi armeabi-v7a --public`. This uses
Rust `armv7-linux-androideabi`, NDK `armv7a-linux-androideabi`, Android API 27
(Android 8.1) and audio. Modern ARM64/x86_64 builds retain API 28. The ARMv7 build
omits the Wasmtime/Cranelift native mod runtime; it does not offer `.hmod` installs.

OpenGL ES is an explicit experimental renderer selection. ARMv7 defaults to it;
existing ARM64 renderer choices remain unchanged. It uses wgpu's GLES backend,
not Vulkan emulation. It needs ES 3.0 or later **and** the texture/shader limits
required by the engine. Vulkan is optional in the APK manifest; a GPU still needs
a suitable supported API. Compiling an ARMv7 APK alone does not establish that a
particular GPU can render the game.

Target device: Fujitsu arrows F-03L. DOCOMO lists Android 8.1.0, a 480x854 display,
Snapdragon 210 and 1 GB RAM. The OS/ABI bridge is only part of compatibility;
GPU limits, available memory and sustained frame pacing need physical testing.
No phone FPS figure or playable-on-F-03L claim is made for this build.

Sources: [DOCOMO developer specifications](https://www.docomo.ne.jp/binary/pdf/service/developer/smart_phone/spec/technical_info_spmode/F-03L.pdf),
[DOCOMO product specifications](https://www.docomo.ne.jp/support/product/f03l/spec.html),
[wgpu supported graphics APIs](https://github.com/gfx-rs/wgpu#supported-platforms).


Upstream integration for this update stops at `29c09b4b`: six commits across the
first three new merge batches. The changed test selector's Windows fixture now
normalizes paths to POSIX form. The paletted-storage optimization is included;
the automatic worker-count change is retained for desktop, while Android keeps
its previous two-core headroom until physical-device profiling. Later upstream
history was fetched but remains outside this build pending separate review.

# Android release packaging

Horizon Android preserves the upstream Git history, MIT copyright and license,
third-party notices and dependency licenses. See [upstream integration](android-upstream.json).
The `android` branch is maintained separately from upstream deployment workflows.

## Build distribution APKs

Build each required architecture with the optimized public mode:

```sh
python tools/android_build.py --abi arm64-v8a --public
python tools/android_build.py --abi armeabi-v7a --public
python tools/android_build.py --abi x86 --public
python tools/android_build.py --abi x86_64 --public
```

Outputs are `.local/android/apk/<abi>/horizon-android-public.apk`.
The build signs and audits each APK. Default development APKs are not release
artifacts: they may contain private resource bundles and enable debugging.

### Payload

The public APK includes the Rust engine, licensed application dependencies and
allowlisted generated Horizon tables. Original Minecraft resource files, JARs,
textures and sounds are not packaged as game resources. The installer downloads
them from Mojang, checks pinned hashes and reconstructs required world data on
the device. Touch textures and their license come from a pinned revision of
Mojang's official sample repository.

Public builds are non-debuggable, remap personal compiler paths, omit automatic
agent-control sockets and ignore `android-launch.json` development overrides.
The payload audit checks packaged paths, original-resource allowlists, recognized
credential patterns and the Android debuggable flag. Its scope is technical
packaging validation, not an exhaustive credential or licensing review.

### Signing and updates

Preserve the private signing key outside Git and release attachments. Android
requires a matching signing certificate and higher `versionCode` for an in-place
update. `versionName` is the visible application version. Update those values in
`packaging/android/build.gradle` and the launcher version together.

Public releases should attach:

- One clearly named APK per supported ABI.
- `SHA256SUMS.txt` covering the exact attached APKs.
- `LICENSE.txt` and `THIRD_PARTY_NOTICES.md`.
- Release notes describing changes, requirements and known compatibility limits.

Include only APKs that passed their final payload/manifest audit. Confirm version,
minimum SDK, ELF architecture, signing certificate and the exact source commit.
Compilation results and device gameplay results should be reported separately.
Account integration remains a development configuration pending project-owned
Microsoft/Mojang registration; do not describe it as an approved integration.

## Source checks

```sh
python -m unittest discover -s tests -p 'test_android*.py'
python tools/check_public_tree.py --history
```

`.githooks` guards public commits and pushes. Never publish `.local/`, prepared
resources, player data, signing keys or account/session files. Agent-authored
commits retain the attribution required by [AGENTS.md](../AGENTS.md).

An optional source export is available without publishing build inputs:

```sh
python tools/export_android_source.py
```

It writes a ZIP and hash manifest under `.local/android/public/`, excluding game
inputs, caches, saves, build products and signing material. Upstream workflows
remain under `.github/upstream-workflows/*.disabled`; the Android source workflow
is manual and uses read-only permissions.

## Upstream maintenance

Fetch and review `upstream/main` before a release. Merge reviewed stable history,
resolve Android module paths and bridges, update `docs/android-upstream.json`,
and run Android checks before pushing. Keep experimental feature clusters and
upstream deployment changes separate from routine Android fixes.

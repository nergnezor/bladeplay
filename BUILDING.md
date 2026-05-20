# Building interact

This file documents a clean, repeatable setup for desktop and Android builds.

## 1. Clone repositories

This project expects blade as a sibling directory:

```bash
cd /path/to/work
git clone https://github.com/nergnezor/interact.git
git clone https://github.com/nergnezor/blade.git
```

Expected layout:

```text
/path/to/work/
  interact/
  blade/
```

## 2. Install required tools

Minimum:

```bash
rustup toolchain install stable
rustup default stable
```

Android extras:

```bash
rustup target add aarch64-linux-android
cargo install cargo-apk
```

Or with Makefile shortcuts:

```bash
make setup-rust-android
make setup-android
```

On Ubuntu, install Android command line tools:

```bash
sudo apt update
sudo apt install -y android-sdk android-sdk-platform-tools android-sdk-build-tools android-sdk-build-tools-common
```

The Makefile auto-detects Android SDK from common locations, including:

- /usr/lib/android-sdk
- ~/Android/Sdk

## 3. Verify environment

From repo root:

```bash
make doctor
```

If something is missing, doctor prints exactly what is absent.

## 4. Desktop build and run

```bash
make build
make run
```

## 5. Hot-reload logic crate only

```bash
make build-logic
```

## 6. Android compile check (no APK packaging)

```bash
make build-android
```

This verifies that Android cross-compilation works.

## 7. Android APK build and run on device

1. Connect phone with USB debugging enabled.
2. Verify device visibility:

```bash
adb devices
```

3. Build APK:

```bash
make apk-build
```

4. Build, install, and launch on connected device:

```bash
make apk-run
```

Note: APK commands run through `.android-runner/Cargo.toml` to avoid `cargo-apk` workspace limitations.

## 8. VS Code integration

- Build task: `Build Android (aarch64)`
- Launch profile: `Android Run on Device`

Both are already configured in `.vscode/tasks.json` and `.vscode/launch.json`.

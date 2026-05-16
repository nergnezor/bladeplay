# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Run the app (must be run from the repo root so relative paths resolve)
cargo run

# Build only (used before launching via VSCode debugger)
cargo build

# Build just the hot-reload logic library
cargo build -p interact-logic

# Run with logging
RUST_LOG=info cargo run
```

There are no tests. The blade dependencies are patched to a local sibling directory `../blade/` — if that path doesn't exist, the build will fail with an unresolved patch error.

## Architecture

The project is split into two crates:

**`interact-logic` (cdylib + rlib)** — the "tweakable" layer, compiled as a shared library. All hot-reloadable logic lives here: sun positions/orbits, sky gradient, environment map pixel generation, and the declarative scene description (`scene_objects()`). Every public entry point is `#[no_mangle] extern "C"`.

**`interact` (binary)** — the host. Owns the GPU, window, and physics simulation. Calls into `interact-logic` via function pointers loaded with `libloading`.

### Hot reload flow

`game.rs::check_hot_reload()` runs every frame:
1. Polls mtime on `interact-logic/src/lib.rs` — if changed, spawns `cargo build -p interact-logic` as a subprocess.
2. `hot_logic::try_reload()` polls mtime on `target/debug/libinteract_logic.so` — if changed, copies it to a numbered file (avoids Linux's refcount hold on the open file) and `dlopen`s it.
3. On successful reload, `scene.reset_suns()` and `engine.reset_accumulation()` are called so the new constants take effect immediately.

The fallback for each `hot_logic::*` function is the statically-linked `interact_logic::*` symbol, so the app works even before the first `dlopen`.

### Scene objects

`scene_objects()` in `interact-logic` returns a `SceneDesc` (array of `ObjectDesc`, max 64). `scene.sync_dynamic()` diffs this against the live `HashMap<u64, DynPhysics>` each frame — adding/removing GPU objects as needed. Object identity is the `id: u64` field; changing `id` removes and re-creates the object.

Reserved IDs: 100 = ground, 101–103 = sun spheres, 104 = ball, 105 = cube. IDs 1–99 are free.

Models are procedural (no .glb files loaded at runtime). `scene.rs::register_models()` builds sphere, cube, torus, plane, and star meshes as `blade_render::ProceduralGeometry` and registers them with the engine at startup. The string keys match the `model` field in `ObjectDesc` (e.g. `"sphere.glb"`).

### Environment map / sky

Every frame, `make_env_pixels()` generates a 1024×512 equirectangular HDR map (`[f32; 3]` per pixel) entirely in CPU Rust and uploads it to the GPU via `engine.set_environment_map_hdr_data()`. This bypasses blade-engine's asset cache (which keys on filename and would never re-upload).

The env-map has three layers per sun: Rayleigh (broad warm glow, blue suppressed), Mie (tight corona), and the sun disk. All use Gaussians in pixel space with a hard circular clip to avoid square-boundary artefacts.

Env-map UV convention: `u = (dir.x.atan2(dir.z) + PI) / (2*PI)` — positive Z maps to the center/front of the map. Suns are placed at negative Z so they appear at the horizon in front of the default camera.

### Physics

Simple Euler integration in `scene.rs::sync_dynamic()` — gravity, floor bounce (RESTITUTION = 0.75), and drag. No physics engine. Mouse drag uses ray–z-plane intersection (`picking::ray_z_plane_hit`): XY follow the mouse, Z depth is adjusted with scroll wheel.

### Rendering

Uses `blade-engine` with a ray-tracing backend (ReSTIR path tracer). The blade dependency is a fork at `github.com/nergnezor/blade`, branch `wayland-niri-fix`, locally patched to `../blade/`. BLAS (bottom-level acceleration structures) must be built for all procedural geometry — this is done inside `blade_render`'s `create_model`.

The `emissive` field in `ObjectDesc` maps to `color_tint[3]` in blade-engine, which is multiplied by 4.0 in the post-processing shader — values above ~0.1 cause strong bloom. Keep `emissive: 0.0` for normally-lit objects.

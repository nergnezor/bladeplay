# interact

En interaktiv 3D-scen med path-tracing, procedural geometri och live hot-reload.

## Kom igång

```bash
cargo run
```

Kräver en lokal kopia av blade i `../blade/` (branch `wayland-niri-fix`).

## Hot reload

Ändra konstanter eller scen-objekt i `interact-logic/src/lib.rs` och spara — appen uppdateras automatiskt inom ~5 sekunder utan omstart.

**Tweakbara konstanter:**
```rust
const SKY_ZENITH:         [f32; 3]  // himmelsfärg uppåt
const SKY_HORIZON:        [f32; 3]  // glöd vid horisonten
const SKY_NADIR:          [f32; 3]  // mark-reflektion nedåt
const SUN_INTENSITY:      f32       // solarnas ljusstyrka
const SUN_RADIUS:         f32       // solarnas storlek (env-map pixlar)
const RAYLEIGH_INTENSITY: f32       // atmosfärisk glöd
const MIE_INTENSITY:      f32       // corona kring sol-disk
```

Lägg till eller ta bort objekt i `scene_objects()` — de dyker upp/försvinner direkt. Objekt-`id` är stabil identitet; ändra `id` för att skapa ett nytt objekt.

## Kontroller

| Input | Åtgärd |
|---|---|
| Vänster klick + drag | Flytta objekt (XY) |
| Scroll (med objekt valt) | Ändra Z-djup |
| WASD / piltangenter | Flyga kameran |
| Scroll (utan objekt) | Zoom |
| Escape | Avsluta |

## Arkitektur

```
interact/                     # binär — GPU, fönster, event loop
  src/
    main.rs                   # winit ApplicationHandler
    game.rs                   # frame loop, hot-reload polling
    scene.rs                  # procedural geometri, fysik, objektsynk
    hot_logic.rs              # dlopen / dlsym wrapper
    picking.rs                # ray casting

interact-logic/               # cdylib + rlib — hot-reloadbar logik
  src/lib.rs                  # solar, sky, scene_objects(), env-map
```

Rendering via [blade](https://github.com/nergnezor/blade) — ReSTIR path tracer på Vulkan.

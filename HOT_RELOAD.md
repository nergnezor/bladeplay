# Hot Reload

Ändra konstanter i `interact-logic/src/lib.rs` och spara — appen laddar om koden automatiskt inom ~5 sekunder utan att starta om.

## Tweakbara konstanter

```rust
// interact-logic/src/lib.rs

const SKY_ZENITH:   [f32; 3] = [0.02, 0.02, 0.5];  // himmelsfärg uppåt
const SKY_HORIZON:  [f32; 3] = [0.25, 0.08, 0.06];  // glöd vid horisonten
const SKY_NADIR:    [f32; 3] = [0.12, 0.06, 0.03];  // mark-reflektion nedåt
const SUN_INTENSITY: f32     = 2.0;                  // solarnas ljusstyrka
const SUN_RADIUS:    i32     = 1;                    // solarnas vinkkelstorlek (pixlar)
const G:             f32     = 80.0;                 // gravitationskonstant
```

## Flöde

```mermaid
sequenceDiagram
    participant Dev as Utvecklare
    participant Src as interact-logic/src/lib.rs
    participant Cargo as cargo build
    participant So as libinteract_logic.so
    participant HotLogic as hot_logic::try_reload()
    participant GPU as blade-engine GPU

    Dev->>Src: sparar ändring
    note over HotLogic: varje frame: mtime-koll på Src
    HotLogic->>Cargo: spawn "cargo build -p interact-logic"
    Cargo->>So: skriver ny .so
    note over HotLogic: varje frame: mtime-koll på .so
    HotLogic->>So: kopierar till libinteract_logic-loaded-N.so
    HotLogic->>HotLogic: dlopen + dlsym → nya funktionspekare
    HotLogic->>HotLogic: frigör gamla .so
    note over HotLogic: nästa frame_count % 30
    HotLogic->>GPU: make_env_pixels() → set_environment_map_hdr_data()
    GPU-->>Dev: uppdaterat ljus syns
```

## Arkitektur

```mermaid
graph TD
    subgraph "interact-logic (cdylib + rlib)"
        SRC[lib.rs<br/>konstanter + logik]
        SRC --> SE["#[no_mangle] step_suns()"]
        SRC --> ME["#[no_mangle] make_env_pixels()"]
    end

    subgraph "interact (binär)"
        HL[hot_logic.rs<br/>Mutex&lt;Option&lt;Loaded&gt;&gt;]
        HL -->|"fn ptr: step_suns"| STEP[scene.step_suns]
        HL -->|"fn ptr: make_env_pixels"| PIXELS[scene.make_env_pixels]
        PIXELS --> ENGINE[engine.set_environment_map_hdr_data]
        ENGINE --> GPU[(GPU texture<br/>Rgba32Float)]
        
        GR[game.rs<br/>check_hot_reload]
        GR -->|mtime .so ändrad| HL
        GR -->|mtime src ändrad| CARGO[cargo build -p interact-logic]
        CARGO -->|ny .so| HL
    end

    SE -.->|dlsym| HL
    ME -.->|dlsym| HL
```

## Varför ingen fil-I/O för env-kartan

`set_environment_map` i blade-engine cachar texturer per sökväg — samma filnamn returnerar alltid den första inlästa versionen. Lösningen är att kringgå cachen helt: `make_env_pixels()` returnerar en `Vec<[f32; 3]>` som laddas upp direkt till GPU med `set_environment_map_hdr_data()`.

## Felsökning

Kontrollera Debug Console (VS Code) för:

| Meddelande | Betydelse |
|---|---|
| `[hot_logic] reloaded interact_logic (counter=1)` | Initial laddning OK |
| `[hot_logic] source changed, spawning cargo build...` | Källfil sparad, bygger |
| `[hot_logic] reloaded interact_logic (counter=N)` | Ny version laddad |
| `[hot_logic] dlopen failed: ...` | .so korrupt eller ABI-mismatch |

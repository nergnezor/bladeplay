use interact_logic::{Sun, SceneDesc};
use std::sync::Mutex;
use std::time::SystemTime;

type StepSunsFn      = unsafe extern "C" fn(&mut [Sun; 3], f32);
type MakeEnvPixelsFn = unsafe extern "C" fn(&[Sun; 3], *mut [f32; 3]);
type SceneObjectsFn  = unsafe extern "C" fn(&mut SceneDesc);
type MakeSunsFn      = unsafe extern "C" fn(&mut [Sun; 3]);

struct Loaded {
    _lib: libloading::Library,
    step_suns: StepSunsFn,
    make_env_pixels: MakeEnvPixelsFn,
    scene_objects: SceneObjectsFn,
    make_suns: MakeSunsFn,
    mtime: SystemTime,
    counter: u64,
}

static LOADED: Mutex<Option<Loaded>> = Mutex::new(None);
static RELOADED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Returns true (once) if the library was reloaded since last call.
pub fn take_reloaded() -> bool {
    RELOADED.swap(false, std::sync::atomic::Ordering::Relaxed)
}

const LIB_PATH: &str = "target/debug/libinteract_logic.so";

pub fn try_reload() {
    let mtime = match std::fs::metadata(LIB_PATH).and_then(|m| m.modified()) {
        Ok(m) => m,
        Err(e) => { eprintln!("[hot_logic] cannot stat {LIB_PATH}: {e}"); return; }
    };

    let (need_reload, counter) = {
        let g = LOADED.lock().unwrap();
        match g.as_ref() {
            Some(l) if l.mtime == mtime => (false, 0),
            Some(l) => (true, l.counter + 1),
            None    => (true, 1),
        }
    };
    if !need_reload { return; }

    let copy_path = format!("target/debug/libinteract_logic-loaded-{counter}.so");
    if let Err(e) = std::fs::copy(LIB_PATH, &copy_path) {
        eprintln!("[hot_logic] copy failed: {e}"); return;
    }

    unsafe {
        let lib = match libloading::Library::new(&copy_path) {
            Ok(l)  => l,
            Err(e) => { eprintln!("[hot_logic] dlopen failed: {e}"); return; }
        };

        macro_rules! sym {
            ($name:literal, $ty:ty) => {
                match lib.get::<$ty>($name) {
                    Ok(s)  => *s,
                    Err(e) => { eprintln!("[hot_logic] symbol {}: {e}", stringify!($name)); return; }
                }
            };
        }

        let step_suns       = sym!(b"step_suns",       StepSunsFn);
        let make_env_pixels = sym!(b"make_env_pixels",  MakeEnvPixelsFn);
        let scene_objects   = sym!(b"scene_objects",    SceneObjectsFn);
        let make_suns       = sym!(b"make_suns",        MakeSunsFn);

        let mut g = LOADED.lock().unwrap();
        let prev_counter = g.as_ref().map(|l| l.counter);
        *g = Some(Loaded { _lib: lib, step_suns, make_env_pixels, scene_objects, make_suns, mtime, counter });
        RELOADED.store(true, std::sync::atomic::Ordering::Relaxed);
        eprintln!("[hot_logic] reloaded interact_logic (counter={counter})");

        if let Some(prev) = prev_counter {
            let _ = std::fs::remove_file(format!("target/debug/libinteract_logic-loaded-{prev}.so"));
        }
    }
}

pub fn step_suns(suns: &mut [Sun; 3], dt: f32) {
    let g = LOADED.lock().unwrap();
    if let Some(l) = g.as_ref() { unsafe { (l.step_suns)(suns, dt) }; }
    else { interact_logic::step_suns(suns, dt); }
}

pub fn make_env_pixels(suns: &[Sun; 3]) -> Vec<[f32; 3]> {
    const W: u32 = interact_logic::ENV_W;
    const H: u32 = interact_logic::ENV_H;
    let mut pixels = vec![[0f32; 3]; (W * H) as usize];
    let g = LOADED.lock().unwrap();
    if let Some(l) = g.as_ref() { unsafe { (l.make_env_pixels)(suns, pixels.as_mut_ptr()) }; }
    else { interact_logic::make_env_pixels(suns, pixels.as_mut_ptr()); }
    pixels
}

pub fn scene_objects() -> SceneDesc {
    let mut out = SceneDesc::new();
    let g = LOADED.lock().unwrap();
    if let Some(l) = g.as_ref() { unsafe { (l.scene_objects)(&mut out) }; }
    else { interact_logic::scene_objects(&mut out); }
    out
}

pub fn make_suns(out: &mut [Sun; 3]) {
    let g = LOADED.lock().unwrap();
    if let Some(l) = g.as_ref() { unsafe { (l.make_suns)(out) }; }
    else { interact_logic::make_suns(out); }
}

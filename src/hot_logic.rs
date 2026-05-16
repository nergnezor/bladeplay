use interact_logic::Sun;
use std::sync::Mutex;
use std::time::SystemTime;

type StepSunsFn = unsafe extern "C" fn(&mut [Sun; 3], f32);
type MakeEnvPixelsFn = unsafe extern "C" fn(&[Sun; 3], *mut [f32; 3]);
type SphereTintFn = unsafe extern "C" fn(&mut [f32; 4]);

struct Loaded {
    _lib: libloading::Library,
    step_suns: StepSunsFn,
    make_env_pixels: MakeEnvPixelsFn,
    sphere_tint: SphereTintFn,
    mtime: SystemTime,
    counter: u64,
}

static LOADED: Mutex<Option<Loaded>> = Mutex::new(None);

const LIB_PATH: &str = "target/debug/libinteract_logic.so";

pub fn try_reload() {
    let mtime = match std::fs::metadata(LIB_PATH).and_then(|m| m.modified()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[hot_logic] cannot stat {LIB_PATH}: {e}");
            return;
        }
    };

    let (need_reload, counter) = {
        let g = LOADED.lock().unwrap();
        match g.as_ref() {
            Some(l) if l.mtime == mtime => (false, 0),
            Some(l) => (true, l.counter + 1),
            None => (true, 1),
        }
    };
    if !need_reload {
        return;
    }

    let copy_path = format!("target/debug/libinteract_logic-loaded-{counter}.so");
    if let Err(e) = std::fs::copy(LIB_PATH, &copy_path) {
        eprintln!("[hot_logic] copy failed: {e}");
        return;
    }

    unsafe {
        let lib = match libloading::Library::new(&copy_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[hot_logic] dlopen failed: {e}");
                return;
            }
        };
        let step_suns_sym: libloading::Symbol<StepSunsFn> = match lib.get(b"step_suns") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[hot_logic] step_suns symbol: {e}");
                return;
            }
        };
        let make_env_pixels_sym: libloading::Symbol<MakeEnvPixelsFn> = match lib.get(b"make_env_pixels") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[hot_logic] make_env_pixels symbol: {e}");
                return;
            }
        };
        let sphere_tint_sym: libloading::Symbol<SphereTintFn> = match lib.get(b"sphere_tint") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[hot_logic] sphere_tint symbol: {e}");
                return;
            }
        };
        let step_suns = *step_suns_sym;
        let make_env_pixels = *make_env_pixels_sym;
        let sphere_tint = *sphere_tint_sym;
        drop(step_suns_sym);
        drop(make_env_pixels_sym);
        drop(sphere_tint_sym);

        let mut g = LOADED.lock().unwrap();
        let prev_counter = g.as_ref().map(|l| l.counter);
        *g = Some(Loaded {
            _lib: lib,
            step_suns,
            make_env_pixels,
            sphere_tint,
            mtime,
            counter,
        });
        eprintln!("[hot_logic] reloaded interact_logic (counter={counter})");

        if let Some(prev) = prev_counter {
            let _ = std::fs::remove_file(format!(
                "target/debug/libinteract_logic-loaded-{prev}.so"
            ));
        }
    }
}

pub fn step_suns(suns: &mut [Sun; 3], dt: f32) {
    let g = LOADED.lock().unwrap();
    if let Some(l) = g.as_ref() {
        unsafe { (l.step_suns)(suns, dt) };
    } else {
        interact_logic::step_suns(suns, dt);
    }
}

pub fn make_env_pixels(suns: &[Sun; 3]) -> Vec<[f32; 3]> {
    const W: u32 = interact_logic::ENV_W;
    const H: u32 = interact_logic::ENV_H;
    let mut pixels = vec![[0f32; 3]; (W * H) as usize];
    let g = LOADED.lock().unwrap();
    if let Some(l) = g.as_ref() {
        unsafe { (l.make_env_pixels)(suns, pixels.as_mut_ptr()) };
    } else {
        interact_logic::make_env_pixels(suns, pixels.as_mut_ptr());
    }
    pixels
}

pub fn sphere_tint() -> [f32; 4] {
    let mut out = [0f32; 4];
    let g = LOADED.lock().unwrap();
    if let Some(l) = g.as_ref() {
        unsafe { (l.sphere_tint)(&mut out) };
    } else {
        interact_logic::sphere_tint(&mut out);
    }
    out
}

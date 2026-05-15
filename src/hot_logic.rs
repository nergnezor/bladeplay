use interact_logic::Sun;
use std::sync::Mutex;
use std::time::SystemTime;

type StepSunsFn = unsafe extern "C" fn(&mut [Sun; 3], f32);
type WriteEnvHdrFn = unsafe extern "C" fn(&[Sun; 3]);

struct Loaded {
    _lib: libloading::Library,
    step_suns: StepSunsFn,
    write_env_hdr: WriteEnvHdrFn,
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
        let write_env_hdr_sym: libloading::Symbol<WriteEnvHdrFn> = match lib.get(b"write_env_hdr") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[hot_logic] write_env_hdr symbol: {e}");
                return;
            }
        };
        let step_suns = *step_suns_sym;
        let write_env_hdr = *write_env_hdr_sym;
        drop(step_suns_sym);
        drop(write_env_hdr_sym);

        let mut g = LOADED.lock().unwrap();
        let prev_counter = g.as_ref().map(|l| l.counter);
        *g = Some(Loaded {
            _lib: lib,
            step_suns,
            write_env_hdr,
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

pub fn write_env_hdr(suns: &[Sun; 3]) {
    let g = LOADED.lock().unwrap();
    if let Some(l) = g.as_ref() {
        unsafe { (l.write_env_hdr)(suns) };
    } else {
        interact_logic::write_env_hdr(suns);
    }
}

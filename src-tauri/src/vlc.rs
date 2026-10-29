//! Minimale, dynamisch geladen LibVLC-koppeling.
//!
//! LibVLC wordt tijdens het draaien geladen (geen link-time afhankelijkheid):
//! zo kan de app een duidelijke fout tonen als de DLL ontbreekt, en blijft de
//! LGPL-scheiding tussen onze code en LibVLC helder.

use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::Path;

pub struct Vlc {
    _lib: Library,
    instance: *mut c_void,
    f: Fns,
}

struct Fns {
    media_new_path: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
    media_release: unsafe extern "C" fn(*mut c_void),
    player_new_from_media: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    player_set_hwnd: unsafe extern "C" fn(*mut c_void, *mut c_void),
    player_play: unsafe extern "C" fn(*mut c_void) -> c_int,
    player_stop: unsafe extern "C" fn(*mut c_void),
    player_release: unsafe extern "C" fn(*mut c_void),
    release: unsafe extern "C" fn(*mut c_void),
    get_version: unsafe extern "C" fn() -> *const c_char,
}

pub struct Player {
    handle: *mut c_void,
}

unsafe impl Send for Vlc {}
unsafe impl Send for Player {}

impl Vlc {
    pub fn load(dir: &Path, args: &[&str]) -> Result<Vlc, String> {
        let dll = dir.join(if cfg!(windows) { "libvlc.dll" } else { "libvlc.so.5" });
        // SAFETY: we laden een bekende bibliotheek en controleren elke symbolen-lookup.
        unsafe {
            #[cfg(windows)]
            let lib_res = libloading::os::windows::Library::load_with_flags(
                &dll,
                0x0000_0008, // LOAD_WITH_ALTERED_SEARCH_PATH: vind libvlccore.dll ernaast
            )
            .map(Library::from);
            #[cfg(not(windows))]
            let lib_res = Library::new(&dll);
            let lib = lib_res
                .map_err(|e| format!("LibVLC kon niet worden geladen ({}): {e}", dll.display()))?;

            macro_rules! sym {
                ($name:literal) => {
                    *lib.get($name).map_err(|e| format!("LibVLC-functie ontbreekt: {e}"))?
                };
            }
            let new: unsafe extern "C" fn(c_int, *const *const c_char) -> *mut c_void = sym!(b"libvlc_new\0");
            let f = Fns {
                media_new_path: sym!(b"libvlc_media_new_path\0"),
                media_release: sym!(b"libvlc_media_release\0"),
                player_new_from_media: sym!(b"libvlc_media_player_new_from_media\0"),
                player_set_hwnd: sym!(b"libvlc_media_player_set_hwnd\0"),
                player_play: sym!(b"libvlc_media_player_play\0"),
                player_stop: sym!(b"libvlc_media_player_stop\0"),
                player_release: sym!(b"libvlc_media_player_release\0"),
                release: sym!(b"libvlc_release\0"),
                get_version: sym!(b"libvlc_get_version\0"),
            };
            let c_args: Vec<CString> = args.iter().map(|a| CString::new(*a).unwrap()).collect();
            let ptrs: Vec<*const c_char> = c_args.iter().map(|a| a.as_ptr()).collect();
            let instance = new(ptrs.len() as c_int, ptrs.as_ptr());
            if instance.is_null() {
                return Err("libvlc_new gaf geen instantie (plugins ontbreken?)".into());
            }
            Ok(Vlc { _lib: lib, instance, f })
        }
    }

    pub fn version(&self) -> String {
        // SAFETY: get_version geeft een statische C-string terug.
        unsafe { std::ffi::CStr::from_ptr((self.f.get_version)()).to_string_lossy().into_owned() }
    }

    /// Opent een lokaal bestand en tekent het in het opgegeven venster (HWND).
    pub fn play_in_window(&self, path: &str, hwnd: *mut c_void) -> Result<Player, String> {
        let c_path = CString::new(path).map_err(|_| "Ongeldig pad".to_string())?;
        // SAFETY: instance en pointers zijn geldig voor de duur van de aanroepen.
        unsafe {
            let media = (self.f.media_new_path)(self.instance, c_path.as_ptr());
            if media.is_null() {
                return Err("Media kon niet worden aangemaakt".into());
            }
            let player = (self.f.player_new_from_media)(media);
            (self.f.media_release)(media);
            if player.is_null() {
                return Err("Speler kon niet worden aangemaakt".into());
            }
            (self.f.player_set_hwnd)(player, hwnd);
            if (self.f.player_play)(player) != 0 {
                (self.f.player_release)(player);
                return Err("Afspelen kon niet starten".into());
            }
            Ok(Player { handle: player })
        }
    }

    pub fn stop_and_release(&self, p: Player) {
        // SAFETY: handle komt uit play_in_window en wordt hier één keer vrijgegeven.
        unsafe {
            (self.f.player_stop)(p.handle);
            (self.f.player_release)(p.handle);
        }
    }
}

impl Drop for Vlc {
    fn drop(&mut self) {
        // SAFETY: instance werd door libvlc_new aangemaakt.
        unsafe { (self.f.release)(self.instance) }
    }
}

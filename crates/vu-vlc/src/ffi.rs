//! Ruwe LibVLC-functies, geladen via `libloading`.

use libloading::Library;
use std::ffi::{c_char, c_float, c_int, c_uint, c_void, CStr};
use std::path::Path;

pub type Instance = *mut c_void;
pub type Media = *mut c_void;
pub type MediaPlayer = *mut c_void;
pub type Equalizer = *mut c_void;

#[repr(C)]
pub struct TrackDescription {
    pub id: c_int,
    pub name: *mut c_char,
    pub next: *mut TrackDescription,
}

#[repr(C)]
pub struct AudioOutputDevice {
    pub next: *mut AudioOutputDevice,
    pub device: *mut c_char,
    pub description: *mut c_char,
}

macro_rules! lib_fns {
    ($( $field:ident : $name:literal => fn($($arg:ty),*) $(-> $ret:ty)? ),* $(,)?) => {
        pub struct Api {
            _lib: Library,
            $( pub $field: unsafe extern "C" fn($($arg),*) $(-> $ret)?, )*
        }

        impl Api {
            pub fn load(path: &Path) -> Result<Api, String> {
                // SAFETY: we laden een bekende bibliotheek; elke symbool-lookup wordt gecontroleerd.
                unsafe {
                    #[cfg(windows)]
                    let lib = libloading::os::windows::Library::load_with_flags(path, 0x0000_0008).map(Library::from);
                    #[cfg(not(windows))]
                    let lib = Library::new(path);
                    let lib = lib.map_err(|e| format!("LibVLC kon niet worden geladen ({}): {e}", path.display()))?;
                    $(
                        let $field: unsafe extern "C" fn($($arg),*) $(-> $ret)? = *lib
                            .get(concat!($name, "\0").as_bytes())
                            .map_err(|e| format!("LibVLC-functie {} ontbreekt: {e}", $name))?;
                    )*
                    Ok(Api { _lib: lib, $( $field, )* })
                }
            }
        }
    };
}

lib_fns! {
    new: "libvlc_new" => fn(c_int, *const *const c_char) -> Instance,
    release: "libvlc_release" => fn(Instance),
    get_version: "libvlc_get_version" => fn() -> *const c_char,
    errmsg: "libvlc_errmsg" => fn() -> *const c_char,

    media_new_path: "libvlc_media_new_path" => fn(Instance, *const c_char) -> Media,
    media_new_location: "libvlc_media_new_location" => fn(Instance, *const c_char) -> Media,
    media_add_option: "libvlc_media_add_option" => fn(Media, *const c_char),
    media_release: "libvlc_media_release" => fn(Media),

    player_new: "libvlc_media_player_new" => fn(Instance) -> MediaPlayer,
    player_release: "libvlc_media_player_release" => fn(MediaPlayer),
    player_set_media: "libvlc_media_player_set_media" => fn(MediaPlayer, Media),
    player_play: "libvlc_media_player_play" => fn(MediaPlayer) -> c_int,
    player_set_pause: "libvlc_media_player_set_pause" => fn(MediaPlayer, c_int),
    player_stop: "libvlc_media_player_stop" => fn(MediaPlayer),
    player_get_state: "libvlc_media_player_get_state" => fn(MediaPlayer) -> c_int,
    player_get_time: "libvlc_media_player_get_time" => fn(MediaPlayer) -> i64,
    player_set_time: "libvlc_media_player_set_time" => fn(MediaPlayer, i64),
    player_get_length: "libvlc_media_player_get_length" => fn(MediaPlayer) -> i64,
    player_get_rate: "libvlc_media_player_get_rate" => fn(MediaPlayer) -> c_float,
    player_set_rate: "libvlc_media_player_set_rate" => fn(MediaPlayer, c_float) -> c_int,
    player_next_frame: "libvlc_media_player_next_frame" => fn(MediaPlayer),
    player_set_equalizer: "libvlc_media_player_set_equalizer" => fn(MediaPlayer, Equalizer) -> c_int,

    audio_set_volume: "libvlc_audio_set_volume" => fn(MediaPlayer, c_int) -> c_int,
    audio_set_mute: "libvlc_audio_set_mute" => fn(MediaPlayer, c_int),
    audio_get_track_description: "libvlc_audio_get_track_description" => fn(MediaPlayer) -> *mut TrackDescription,
    audio_get_track: "libvlc_audio_get_track" => fn(MediaPlayer) -> c_int,
    audio_set_track: "libvlc_audio_set_track" => fn(MediaPlayer, c_int) -> c_int,
    audio_get_delay: "libvlc_audio_get_delay" => fn(MediaPlayer) -> i64,
    audio_set_delay: "libvlc_audio_set_delay" => fn(MediaPlayer, i64) -> c_int,
    audio_output_device_enum: "libvlc_audio_output_device_enum" => fn(MediaPlayer) -> *mut AudioOutputDevice,
    audio_output_device_list_release: "libvlc_audio_output_device_list_release" => fn(*mut AudioOutputDevice),
    audio_output_device_set: "libvlc_audio_output_device_set" => fn(MediaPlayer, *const c_char, *const c_char),

    video_get_spu_description: "libvlc_video_get_spu_description" => fn(MediaPlayer) -> *mut TrackDescription,
    video_get_spu: "libvlc_video_get_spu" => fn(MediaPlayer) -> c_int,
    video_set_spu: "libvlc_video_set_spu" => fn(MediaPlayer, c_int) -> c_int,
    video_set_subtitle_file: "libvlc_video_set_subtitle_file" => fn(MediaPlayer, *const c_char) -> c_int,
    video_get_spu_delay: "libvlc_video_get_spu_delay" => fn(MediaPlayer) -> i64,
    video_set_spu_delay: "libvlc_video_set_spu_delay" => fn(MediaPlayer, i64) -> c_int,
    video_take_snapshot: "libvlc_video_take_snapshot" => fn(MediaPlayer, c_uint, *const c_char, c_uint, c_uint) -> c_int,
    video_set_aspect_ratio: "libvlc_video_set_aspect_ratio" => fn(MediaPlayer, *const c_char),
    video_set_crop_geometry: "libvlc_video_set_crop_geometry" => fn(MediaPlayer, *const c_char),
    video_set_adjust_int: "libvlc_video_set_adjust_int" => fn(MediaPlayer, c_uint, c_int),
    video_set_adjust_float: "libvlc_video_set_adjust_float" => fn(MediaPlayer, c_uint, c_float),
    video_set_deinterlace: "libvlc_video_set_deinterlace" => fn(MediaPlayer, *const c_char),
    video_get_size: "libvlc_video_get_size" => fn(MediaPlayer, c_uint, *mut c_uint, *mut c_uint) -> c_int,

    track_description_list_release: "libvlc_track_description_list_release" => fn(*mut TrackDescription),
    equalizer_new: "libvlc_audio_equalizer_new" => fn() -> Equalizer,
    equalizer_release: "libvlc_audio_equalizer_release" => fn(Equalizer),
    equalizer_set_preamp: "libvlc_audio_equalizer_set_preamp" => fn(Equalizer, c_float) -> c_int,
    equalizer_set_amp_at_index: "libvlc_audio_equalizer_set_amp_at_index" => fn(Equalizer, c_float, c_uint) -> c_int,
}

#[cfg(windows)]
pub mod platform {
    use super::*;
    pub type SetWindow = unsafe extern "C" fn(MediaPlayer, *mut c_void);
    pub const SYMBOL: &str = "libvlc_media_player_set_hwnd";
}

#[cfg(not(windows))]
pub mod platform {
    use super::*;
    pub type SetWindow = unsafe extern "C" fn(MediaPlayer, u32);
    pub const SYMBOL: &str = "libvlc_media_player_set_xwindow";
}

impl Api {
    /// Haalt de venster-koppeling op (HWND op Windows, X11-venster op Linux).
    pub fn set_window_fn(&self) -> Result<platform::SetWindow, String> {
        // SAFETY: symbool uit dezelfde, nog geladen bibliotheek.
        unsafe {
            self._lib
                .get::<platform::SetWindow>(format!("{}\0", platform::SYMBOL).as_bytes())
                .map(|f| *f)
                .map_err(|e| format!("LibVLC-functie {} ontbreekt: {e}", platform::SYMBOL))
        }
    }
}

/// Kopieert een C-string die LibVLC bezit.
///
/// # Safety
/// `p` is null of wijst naar een geldige, nul-getermineerde string.
pub unsafe fn c_str(p: *const c_char) -> Option<String> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_string_lossy().into_owned())
    }
}

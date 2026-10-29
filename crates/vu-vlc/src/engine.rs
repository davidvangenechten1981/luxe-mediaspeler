//! `VlcEngine`: implementeert `vu_core::player::session::Engine` op LibVLC 3.x.

use crate::ffi::{self, Api, Instance, MediaPlayer};
use std::ffi::{c_char, c_int, c_uint, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use vu_core::audio::{EqConfig, BAND_FREQS_HZ};
use vu_core::error::{CoreError, Result};
use vu_core::player::session::{Engine, EngineStatus, OpenOptions, PlaybackState, Track};

#[derive(Debug, Clone, Default)]
pub struct VlcConfig {
    /// Map met `libvlc.dll` (Windows) of de systeembibliotheek als `None`.
    pub library_dir: Option<PathBuf>,
    /// Extra opties voor `libvlc_new`; alleen gebruiken in tests (bijv. `--vout=dummy`).
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rotation {
    #[default]
    None,
    Rotate90,
    Rotate180,
    Rotate270,
    FlipHorizontal,
    FlipVertical,
}

impl Rotation {
    fn option(self) -> Option<&'static str> {
        match self {
            Rotation::None => None,
            Rotation::Rotate90 => Some("90"),
            Rotation::Rotate180 => Some("180"),
            Rotation::Rotate270 => Some("270"),
            Rotation::FlipHorizontal => Some("hflip"),
            Rotation::FlipVertical => Some("vflip"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visualization {
    #[default]
    Spectrum,
    Spectrometer,
    Scope,
    VuMeter,
}

impl Visualization {
    fn effect(self) -> &'static str {
        match self {
            Visualization::Spectrum => "spectrum",
            Visualization::Spectrometer => "spectrometer",
            Visualization::Scope => "scope",
            Visualization::VuMeter => "vuMeter",
        }
    }
}

/// Beeldinstellingen van LibVLC (`libvlc_video_adjust_option_t`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adjust {
    Contrast,
    Brightness,
    Hue,
    Saturation,
    Gamma,
}

impl Adjust {
    fn id(self) -> c_uint {
        match self {
            Adjust::Contrast => 1,
            Adjust::Brightness => 2,
            Adjust::Hue => 3,
            Adjust::Saturation => 4,
            Adjust::Gamma => 5,
        }
    }
    /// Toegestaan bereik volgens LibVLC.
    pub fn range(self) -> (f32, f32) {
        match self {
            Adjust::Contrast | Adjust::Brightness | Adjust::Saturation => (0.0, 2.0),
            Adjust::Hue => (-180.0, 180.0),
            Adjust::Gamma => (0.01, 10.0),
        }
    }
}

pub struct VlcEngine {
    api: Arc<Api>,
    instance: Instance,
    player: MediaPlayer,
    opened: bool,
    last_open: Option<(String, OpenOptions)>,
    hw_decoding: String,
    rotation: Rotation,
    visualization: Visualization,
    normalize: bool,
    eq: Option<EqConfig>,
    /// Het volume dat wij hebben ingesteld. Alleen wij wijzigen het softwarevolume, en LibVLC
    /// meldt het niet betrouwbaar terug zolang de geluidsuitvoer niet draait.
    volume_hint: AtomicU32,
    muted_hint: AtomicBool,
}

// SAFETY: LibVLC-aanroepen zijn thread-safe; de pointers worden alleen via deze struct gebruikt.
unsafe impl Send for VlcEngine {}

fn cstr(s: &str) -> Result<CString> {
    CString::new(s).map_err(|_| CoreError::Invalid("Tekst bevat een ongeldig teken".into()))
}

fn is_url(s: &str) -> bool {
    s.split_once("://").is_some_and(|(scheme, _)| !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c)))
}

impl VlcEngine {
    pub fn new(cfg: &VlcConfig) -> Result<VlcEngine> {
        let lib_name = if cfg!(windows) { "libvlc.dll" } else { "libvlc.so.5" };
        let lib_path = cfg.library_dir.as_ref().map(|d| d.join(lib_name)).unwrap_or_else(|| PathBuf::from(lib_name));
        if let Some(dir) = &cfg.library_dir {
            // Zorg dat LibVLC zijn plugins naast de DLL vindt.
            std::env::set_var("VLC_PLUGIN_PATH", dir.join("plugins"));
        }
        let api = Arc::new(Api::load(&lib_path).map_err(CoreError::Engine)?);

        let mut args: Vec<String> = vec!["--quiet".into(), "--no-video-title-show".into(), "--no-osd".into(), "--no-snapshot-preview".into()];
        args.extend(cfg.extra_args.iter().cloned());
        let c_args: Vec<CString> = args.iter().map(|a| cstr(a)).collect::<Result<_>>()?;
        let ptrs: Vec<*const c_char> = c_args.iter().map(|a| a.as_ptr()).collect();
        // SAFETY: ptrs verwijzen naar c_args, die tijdens de aanroep leven.
        let instance = unsafe { (api.new)(ptrs.len() as c_int, ptrs.as_ptr()) };
        if instance.is_null() {
            return Err(CoreError::Engine(format!("LibVLC kon niet starten: {}", Self::errmsg(&api))));
        }
        // SAFETY: instance is geldig.
        let player = unsafe { (api.player_new)(instance) };
        if player.is_null() {
            // SAFETY: instance werd hierboven aangemaakt.
            unsafe { (api.release)(instance) };
            return Err(CoreError::Engine("LibVLC kon geen speler aanmaken".into()));
        }
        Ok(VlcEngine {
            api,
            instance,
            player,
            opened: false,
            last_open: None,
            hw_decoding: "any".into(),
            rotation: Rotation::None,
            visualization: Visualization::Spectrum,
            normalize: false,
            eq: None,
            volume_hint: AtomicU32::new(100),
            muted_hint: AtomicBool::new(false),
        })
    }

    fn errmsg(api: &Api) -> String {
        // SAFETY: errmsg geeft null of een geldige string terug.
        unsafe { ffi::c_str((api.errmsg)()) }.unwrap_or_else(|| "onbekende fout".into())
    }

    pub fn version(&self) -> String {
        // SAFETY: get_version geeft een statische string.
        unsafe { ffi::c_str((self.api.get_version)()) }.unwrap_or_default()
    }

    /// Laat de video tekenen in een venster (HWND op Windows, X11-id op Linux).
    pub fn set_window(&mut self, handle: u64) -> Result<()> {
        let f = self.api.set_window_fn().map_err(CoreError::Engine)?;
        // SAFETY: speler is geldig; handle is een venster van de aanroeper.
        unsafe {
            #[cfg(windows)]
            f(self.player, handle as usize as *mut std::ffi::c_void);
            #[cfg(not(windows))]
            f(self.player, handle as u32);
        }
        Ok(())
    }

    // ---------- instellingen die bij het openen worden toegepast ----------

    /// "any", "none", "d3d11va" of "dxva2". Geldt vanaf de volgende keer openen.
    pub fn set_hw_decoding(&mut self, mode: &str) -> Result<()> {
        if !["any", "none", "d3d11va", "dxva2"].contains(&mode) {
            return Err(CoreError::Invalid(format!("Onbekende decodeermodus '{mode}'")));
        }
        self.hw_decoding = mode.to_string();
        Ok(())
    }

    /// Opnieuw openen op de huidige positie, zodat een mediaoptie (bijv. rotatie) van kracht wordt.
    fn reopen_here(&mut self) -> Result<()> {
        let Some((loc, mut opts)) = self.last_open.clone() else { return Ok(()) };
        let st = self.status();
        let was_paused = st.state == PlaybackState::Paused;
        opts.start_ms = st.time_ms;
        self.open(&loc, &opts)?;
        self.play()?;
        if was_paused {
            self.pause()?;
        }
        Ok(())
    }

    pub fn set_rotation(&mut self, r: Rotation) -> Result<()> {
        if self.rotation != r {
            self.rotation = r;
            self.reopen_here()?;
        }
        Ok(())
    }

    pub fn set_visualization(&mut self, v: Visualization) -> Result<()> {
        if self.visualization != v {
            self.visualization = v;
            self.reopen_here()?;
        }
        Ok(())
    }

    pub fn set_normalize(&mut self, on: bool) -> Result<()> {
        if self.normalize != on {
            self.normalize = on;
            self.reopen_here()?;
        }
        Ok(())
    }

    // ---------- beeld ----------

    /// Beeldverhouding zoals "16:9", of `None` voor het origineel.
    pub fn set_aspect_ratio(&mut self, ratio: Option<&str>) -> Result<()> {
        if let Some(r) = ratio {
            let ok = r.split_once(':').is_some_and(|(a, b)| a.parse::<u32>().is_ok_and(|v| v > 0) && b.parse::<u32>().is_ok_and(|v| v > 0));
            if !ok {
                return Err(CoreError::Invalid(format!("Ongeldige beeldverhouding '{r}' (gebruik bv. 16:9)")));
            }
        }
        let c = ratio.map(cstr).transpose()?;
        // SAFETY: pointer is null of geldig voor de duur van de aanroep.
        unsafe { (self.api.video_set_aspect_ratio)(self.player, c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr())) };
        Ok(())
    }

    /// Bijsnijden naar een verhouding zoals "16:9", of `None` voor geen bijsnijding.
    pub fn set_crop(&mut self, ratio: Option<&str>) -> Result<()> {
        if let Some(r) = ratio {
            let ok = r.split_once(':').is_some_and(|(a, b)| a.parse::<u32>().is_ok_and(|v| v > 0) && b.parse::<u32>().is_ok_and(|v| v > 0));
            if !ok {
                return Err(CoreError::Invalid(format!("Ongeldige bijsnijding '{r}' (gebruik bv. 16:9)")));
            }
        }
        let c = ratio.map(cstr).transpose()?;
        // SAFETY: zie hierboven.
        unsafe { (self.api.video_set_crop_geometry)(self.player, c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr())) };
        Ok(())
    }

    pub fn set_adjust(&mut self, option: Adjust, value: f32) -> Result<()> {
        let (lo, hi) = option.range();
        if !value.is_finite() || value < lo || value > hi {
            return Err(CoreError::Invalid(format!("Waarde {value} ligt buiten het bereik {lo}..{hi}")));
        }
        // SAFETY: speler is geldig. Optie 0 = inschakelen van de beeldfilter.
        unsafe {
            (self.api.video_set_adjust_int)(self.player, 0, 1);
            (self.api.video_set_adjust_float)(self.player, option.id(), value);
        }
        Ok(())
    }

    /// Deinterlace-modus: "off", "on", "blend", "bob", "linear", "x", "yadif", "yadif2x".
    pub fn set_deinterlace(&mut self, mode: &str) -> Result<()> {
        if !["off", "on", "blend", "bob", "linear", "x", "yadif", "yadif2x"].contains(&mode) {
            return Err(CoreError::Invalid(format!("Onbekende deinterlace-modus '{mode}'")));
        }
        let c = cstr(mode)?;
        // SAFETY: c leeft tijdens de aanroep.
        unsafe { (self.api.video_set_deinterlace)(self.player, c.as_ptr()) };
        Ok(())
    }

    pub fn video_size(&self) -> Option<(u32, u32)> {
        let (mut w, mut h): (c_uint, c_uint) = (0, 0);
        // SAFETY: w/h zijn geldige uit-parameters.
        let r = unsafe { (self.api.video_get_size)(self.player, 0, &mut w, &mut h) };
        (r == 0 && w > 0 && h > 0).then_some((w, h))
    }

    // ---------- audioapparaten ----------

    pub fn list_audio_devices(&self) -> Vec<(String, String)> {
        let mut out = vec![];
        // SAFETY: de lijst wordt na het lezen vrijgegeven; namen worden meteen gekopieerd.
        unsafe {
            let head = (self.api.audio_output_device_enum)(self.player);
            let mut cur = head;
            while !cur.is_null() {
                let id = ffi::c_str((*cur).device).unwrap_or_default();
                let desc = ffi::c_str((*cur).description).unwrap_or_default();
                out.push((id, desc));
                cur = (*cur).next;
            }
            if !head.is_null() {
                (self.api.audio_output_device_list_release)(head);
            }
        }
        out
    }

    /// Kiest een apparaat uit `list_audio_devices`. Een onbekend id wordt geweigerd.
    pub fn set_audio_device(&mut self, id: &str) -> Result<()> {
        if !self.list_audio_devices().iter().any(|(d, _)| d == id) {
            return Err(CoreError::NotFound(format!("audioapparaat '{id}'")));
        }
        let c = cstr(id)?;
        // SAFETY: c leeft tijdens de aanroep; null = standaard audio-uitvoermodule.
        unsafe { (self.api.audio_output_device_set)(self.player, std::ptr::null(), c.as_ptr()) };
        Ok(())
    }

    // ---------- hulp ----------

    fn tracks(&self, list: *mut ffi::TrackDescription) -> Vec<Track> {
        let mut out = vec![];
        // SAFETY: de lijst komt van LibVLC; namen worden gekopieerd voordat de lijst wordt vrijgegeven.
        unsafe {
            let mut cur = list;
            while !cur.is_null() {
                let id = (*cur).id;
                if id >= 0 {
                    out.push(Track { id, name: ffi::c_str((*cur).name).unwrap_or_default(), language: None });
                }
                cur = (*cur).next;
            }
            if !list.is_null() {
                (self.api.track_description_list_release)(list);
            }
        }
        out
    }

    fn check(&self, code: c_int, what: &str) -> Result<()> {
        if code == 0 {
            Ok(())
        } else {
            Err(CoreError::Engine(format!("{what} mislukte: {}", Self::errmsg(&self.api))))
        }
    }
}

impl Drop for VlcEngine {
    fn drop(&mut self) {
        // SAFETY: speler en instantie zijn van ons en worden één keer vrijgegeven, speler eerst.
        unsafe {
            (self.api.player_stop)(self.player);
            (self.api.player_release)(self.player);
            (self.api.release)(self.instance);
        }
    }
}

impl Engine for VlcEngine {
    fn open(&mut self, location: &str, opts: &OpenOptions) -> Result<()> {
        let url = is_url(location);
        if !url && !Path::new(location).is_file() {
            return Err(CoreError::NotFound(location.to_string()));
        }
        let c_loc = cstr(location)?;
        // SAFETY: instance is geldig; c_loc leeft tijdens de aanroep.
        let media = unsafe {
            if url { (self.api.media_new_location)(self.instance, c_loc.as_ptr()) } else { (self.api.media_new_path)(self.instance, c_loc.as_ptr()) }
        };
        if media.is_null() {
            return Err(CoreError::Engine(format!("Media kon niet worden aangemaakt: {}", Self::errmsg(&self.api))));
        }
        let mut options: Vec<String> = vec![format!(":avcodec-hw={}", self.hw_decoding)];
        if opts.start_ms > 0 {
            options.push(format!(":start-time={:.3}", opts.start_ms as f64 / 1000.0));
        }
        if let Some(ms) = opts.network_caching_ms {
            options.push(format!(":network-caching={ms}"));
        }
        if opts.audio_visual {
            options.push(":audio-visual=visual".into());
            options.push(format!(":effect-list={}", self.visualization.effect()));
        }
        if let Some(t) = self.rotation.option() {
            options.push(":video-filter=transform".into());
            options.push(format!(":transform-type={t}"));
        }
        if self.normalize {
            options.push(":audio-filter=normvol".into());
        }
        for o in &options {
            let c = cstr(o)?;
            // SAFETY: media is geldig; LibVLC kopieert de optie.
            unsafe { (self.api.media_add_option)(media, c.as_ptr()) };
        }
        // SAFETY: speler en media zijn geldig; media mag direct worden vrijgegeven na set_media.
        unsafe {
            (self.api.player_set_media)(self.player, media);
            (self.api.media_release)(media);
            (self.api.audio_set_volume)(self.player, self.volume_hint.load(Ordering::Relaxed) as c_int);
            (self.api.audio_set_mute)(self.player, self.muted_hint.load(Ordering::Relaxed) as c_int);
        }
        self.opened = true;
        self.last_open = Some((location.to_string(), opts.clone()));
        Ok(())
    }

    fn play(&mut self) -> Result<()> {
        // SAFETY: speler is geldig.
        unsafe {
            if (self.api.player_get_state)(self.player) == 4 {
                (self.api.player_set_pause)(self.player, 0);
                return Ok(());
            }
            let r = (self.api.player_play)(self.player);
            self.check(r, "Afspelen starten")
        }
    }

    fn pause(&mut self) -> Result<()> {
        // SAFETY: speler is geldig.
        unsafe { (self.api.player_set_pause)(self.player, 1) };
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        // SAFETY: speler is geldig.
        unsafe { (self.api.player_stop)(self.player) };
        Ok(())
    }

    fn seek(&mut self, ms: u64) -> Result<()> {
        // SAFETY: speler is geldig.
        unsafe { (self.api.player_set_time)(self.player, ms as i64) };
        Ok(())
    }

    fn set_volume(&mut self, percent: u32) -> Result<()> {
        let v = percent.min(200);
        self.volume_hint.store(v, Ordering::Relaxed);
        // SAFETY: speler is geldig. Mislukt dit omdat de geluidsuitvoer nog niet draait,
        // dan wordt het onthouden volume bij het (her)openen opnieuw toegepast.
        unsafe { (self.api.audio_set_volume)(self.player, v as c_int) };
        Ok(())
    }

    fn set_mute(&mut self, muted: bool) -> Result<()> {
        self.muted_hint.store(muted, Ordering::Relaxed);
        // SAFETY: speler is geldig.
        unsafe { (self.api.audio_set_mute)(self.player, muted as c_int) };
        Ok(())
    }

    fn set_rate(&mut self, rate: f32) -> Result<()> {
        // SAFETY: speler is geldig.
        let r = unsafe { (self.api.player_set_rate)(self.player, rate) };
        self.check(r, "Snelheid instellen")
    }

    fn set_audio_track(&mut self, id: i32) -> Result<()> {
        // SAFETY: speler is geldig.
        let r = unsafe { (self.api.audio_set_track)(self.player, id) };
        self.check(r, "Audiospoor kiezen")
    }

    fn set_subtitle_track(&mut self, id: Option<i32>) -> Result<()> {
        // SAFETY: speler is geldig; -1 schakelt ondertitels uit.
        let r = unsafe { (self.api.video_set_spu)(self.player, id.unwrap_or(-1)) };
        self.check(r, "Ondertitelspoor kiezen")
    }

    fn load_subtitle_file(&mut self, path: &Path) -> Result<()> {
        let c = cstr(&path.to_string_lossy())?;
        // SAFETY: c leeft tijdens de aanroep. LibVLC geeft 1 terug bij succes.
        let ok = unsafe { (self.api.video_set_subtitle_file)(self.player, c.as_ptr()) };
        if ok == 0 {
            return Err(CoreError::Engine(format!("Ondertitelbestand kon niet worden geladen: {}", path.display())));
        }
        Ok(())
    }

    fn set_audio_delay_ms(&mut self, ms: i64) -> Result<()> {
        // SAFETY: speler is geldig; LibVLC rekent in microseconden.
        let r = unsafe { (self.api.audio_set_delay)(self.player, ms * 1000) };
        self.check(r, "Audiovertraging instellen")
    }

    fn set_subtitle_delay_ms(&mut self, ms: i64) -> Result<()> {
        // SAFETY: zie hierboven.
        let r = unsafe { (self.api.video_set_spu_delay)(self.player, ms * 1000) };
        self.check(r, "Ondertitelvertraging instellen")
    }

    fn next_frame(&mut self) -> Result<()> {
        // SAFETY: speler is geldig.
        unsafe { (self.api.player_next_frame)(self.player) };
        Ok(())
    }

    fn take_snapshot(&mut self, path: &Path) -> Result<()> {
        let c = cstr(&path.to_string_lossy())?;
        // SAFETY: c leeft tijdens de aanroep; breedte/hoogte 0 = oorspronkelijke grootte.
        let r = unsafe { (self.api.video_take_snapshot)(self.player, 0, c.as_ptr(), 0, 0) };
        self.check(r, "Screenshot maken")
    }

    fn set_equalizer(&mut self, eq: Option<&EqConfig>) -> Result<()> {
        match eq {
            None => {
                // SAFETY: null schakelt de equalizer uit.
                let r = unsafe { (self.api.player_set_equalizer)(self.player, std::ptr::null_mut()) };
                self.eq = None;
                self.check(r, "Equalizer uitschakelen")
            }
            Some(cfg) => {
                cfg.validate()?;
                // SAFETY: de equalizer wordt door LibVLC gekopieerd bij set_equalizer en daarna vrijgegeven.
                unsafe {
                    let e = (self.api.equalizer_new)();
                    if e.is_null() {
                        return Err(CoreError::Invalid("Equalizer kon niet worden aangemaakt".into()));
                    }
                    (self.api.equalizer_set_preamp)(e, cfg.preamp_db);
                    for (i, g) in cfg.bands_db.iter().enumerate().take(BAND_FREQS_HZ.len()) {
                        (self.api.equalizer_set_amp_at_index)(e, *g, i as c_uint);
                    }
                    let r = (self.api.player_set_equalizer)(self.player, e);
                    (self.api.equalizer_release)(e);
                    self.eq = Some(cfg.clone());
                    self.check(r, "Equalizer toepassen")
                }
            }
        }
    }

    fn status(&self) -> EngineStatus {
        // SAFETY: alle aanroepen gebruiken de geldige speler.
        unsafe {
            let raw = (self.api.player_get_state)(self.player);
            let state = match raw {
                0 => {
                    if self.opened { PlaybackState::Opening } else { PlaybackState::Idle }
                }
                1 => PlaybackState::Opening,
                2 => PlaybackState::Buffering,
                3 => PlaybackState::Playing,
                4 => PlaybackState::Paused,
                5 => PlaybackState::Stopped,
                6 => PlaybackState::Ended,
                _ => PlaybackState::Error,
            };
            let audio_tracks = self.tracks((self.api.audio_get_track_description)(self.player));
            let subtitle_tracks = self.tracks((self.api.video_get_spu_description)(self.player));
            let cur_audio = (self.api.audio_get_track)(self.player);
            let cur_spu = (self.api.video_get_spu)(self.player);
            EngineStatus {
                state,
                time_ms: (self.api.player_get_time)(self.player).max(0) as u64,
                length_ms: (self.api.player_get_length)(self.player).max(0) as u64,
                volume: self.volume_hint.load(Ordering::Relaxed),
                muted: self.muted_hint.load(Ordering::Relaxed),
                rate: (self.api.player_get_rate)(self.player),
                audio_tracks,
                current_audio: (cur_audio >= 0).then_some(cur_audio),
                subtitle_tracks,
                current_subtitle: (cur_spu >= 0).then_some(cur_spu),
                audio_delay_ms: (self.api.audio_get_delay)(self.player) / 1000,
                subtitle_delay_ms: (self.api.video_get_spu_delay)(self.player) / 1000,
                buffering_percent: None,
                error: (state == PlaybackState::Error).then(|| Self::errmsg(&self.api)),
            }
        }
    }
}

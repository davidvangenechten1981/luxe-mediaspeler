//! Windows-mediabediening (SMTC): titel en knoppen in het volumepaneel en op het vergrendelscherm.
//! De logica (wat tonen, wat doen de knoppen) staat in `vu_app::media_session` en is getest; dit bestand
//! koppelt die alleen aan Windows en kan alleen op Windows worden uitgeprobeerd.

use std::sync::{Mutex, OnceLock};
use vu_app::media_session::{display_from, MediaDisplay, MediaKey, MediaStatus};
use windows::core::HSTRING;
use windows::Foundation::TypedEventHandler;
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls, SystemMediaTransportControlsButton,
    SystemMediaTransportControlsButtonPressedEventArgs,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::WinRT::ISystemMediaTransportControlsInterop;

struct Smtc {
    controls: SystemMediaTransportControls,
    last: Option<MediaDisplay>,
}

// SAFETY: de SMTC-objecten zijn agile WinRT-objecten; alle toegang gaat via de Mutex hieronder.
unsafe impl Send for Smtc {}

static SMTC: OnceLock<Mutex<Smtc>> = OnceLock::new();
static STATUS: Mutex<MediaStatus> = Mutex::new(MediaStatus::Stopped);

fn key_of(button: SystemMediaTransportControlsButton) -> Option<MediaKey> {
    Some(match button {
        SystemMediaTransportControlsButton::Play => MediaKey::Play,
        SystemMediaTransportControlsButton::Pause => MediaKey::Pause,
        SystemMediaTransportControlsButton::Stop => MediaKey::Stop,
        SystemMediaTransportControlsButton::Next => MediaKey::Next,
        SystemMediaTransportControlsButton::Previous => MediaKey::Previous,
        _ => return None,
    })
}

/// Koppelt SMTC aan het hoofdvenster. `on_key` wordt aangeroepen met de knop en de huidige status.
pub fn init(hwnd: isize, on_key: impl Fn(MediaKey, MediaStatus) + Send + Sync + 'static) -> windows::core::Result<()> {
    let interop = windows::core::factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()?;
    // SAFETY: `hwnd` is het geldige vensterhandvat van het hoofdvenster, dat de hele levensduur van de app bestaat.
    let controls: SystemMediaTransportControls = unsafe { interop.GetForWindow(HWND(hwnd as *mut std::ffi::c_void)) }?;
    controls.SetIsEnabled(true)?;
    controls.SetIsPlayEnabled(true)?;
    controls.SetIsPauseEnabled(true)?;
    controls.SetIsStopEnabled(true)?;
    controls.SetIsNextEnabled(true)?;
    controls.SetIsPreviousEnabled(true)?;
    controls.ButtonPressed(&TypedEventHandler::new(
        move |_: &Option<SystemMediaTransportControls>, args: &Option<SystemMediaTransportControlsButtonPressedEventArgs>| {
            if let Some(key) = args.as_ref().and_then(|a| a.Button().ok()).and_then(key_of) {
                let status = STATUS.lock().map(|s| *s).unwrap_or(MediaStatus::Stopped);
                on_key(key, status);
            }
            Ok(())
        },
    ))?;
    let _ = SMTC.set(Mutex::new(Smtc { controls, last: None }));
    Ok(())
}

/// Werkt titel en status bij op basis van de statusmelding van de speler; doet niets als er niets veranderd is.
pub fn on_player_state(payload: &serde_json::Value) {
    let display = display_from(payload);
    if let Ok(mut s) = STATUS.lock() {
        *s = display.status;
    }
    let Some(smtc) = SMTC.get() else { return };
    let Ok(mut smtc) = smtc.lock() else { return };
    if smtc.last.as_ref() == Some(&display) {
        return;
    }
    let _ = apply(&smtc.controls, &display);
    smtc.last = Some(display);
}

fn apply(controls: &SystemMediaTransportControls, d: &MediaDisplay) -> windows::core::Result<()> {
    controls.SetPlaybackStatus(match d.status {
        MediaStatus::Playing => MediaPlaybackStatus::Playing,
        MediaStatus::Paused => MediaPlaybackStatus::Paused,
        MediaStatus::Stopped => MediaPlaybackStatus::Stopped,
    })?;
    let updater = controls.DisplayUpdater()?;
    match &d.title {
        Some(title) => {
            updater.SetType(MediaPlaybackType::Music)?;
            updater.MusicProperties()?.SetTitle(&HSTRING::from(title.as_str()))?;
            updater.Update()?;
        }
        None => updater.ClearAll()?,
    }
    Ok(())
}

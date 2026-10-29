//! Windows-integratie (bestandsassociaties en contextmenu), uitsluitend op expliciet verzoek
//! van de gebruiker. Alles gaat in HKEY_CURRENT_USER: geen beheerdersrechten, en volledig terug te draaien.

use serde::Serialize;

#[cfg_attr(not(windows), allow(dead_code))]
pub const MEDIA_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "webm", "mpg", "mpeg", "ts", "m2ts", "wmv", "flv", "ogv", "m4v", "mp3", "aac", "flac", "wav", "ogg",
    "opus", "m4a", "wma", "aiff", "m3u", "m3u8",
];

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct IntegrationStatus {
    pub supported: bool,
    pub open_with_registered: bool,
    pub context_menu_registered: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(not(windows), allow(dead_code))]
pub struct Identity {
    pub prog_id: String,
    pub app_key: String,
    pub app_name: String,
    pub verb: String,
}

impl Identity {
    pub fn production() -> Identity {
        Identity {
            prog_id: "VLCUltimate.Media".into(),
            app_key: "VLCUltimate".into(),
            app_name: "VLC Ultimate".into(),
            verb: "VLCUltimate.Play".into(),
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::io;
    use std::path::Path;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    fn classes() -> io::Result<RegKey> {
        RegKey::predef(HKEY_CURRENT_USER).create_subkey("Software\\Classes").map(|(k, _)| k)
    }

    pub fn status(id: &Identity) -> IntegrationStatus {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let open_with = hkcu.open_subkey_with_flags(format!("Software\\Classes\\{}", id.prog_id), KEY_READ).is_ok()
            && hkcu.open_subkey_with_flags(format!("Software\\{}\\Capabilities", id.app_key), KEY_READ).is_ok();
        let context =
            hkcu.open_subkey_with_flags(format!("Software\\Classes\\SystemFileAssociations\\.mp4\\shell\\{}", id.verb), KEY_READ).is_ok();
        IntegrationStatus { supported: true, open_with_registered: open_with, context_menu_registered: context }
    }

    /// Maakt de app beschikbaar in "Openen met" en in de lijst met standaard-apps van Windows.
    /// Windows laat niet toe dat een app zichzelf stil als standaard instelt; de gebruiker kiest dat zelf.
    pub fn set_open_with(id: &Identity, exe: &Path, enable: bool) -> io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let classes = classes()?;
        if !enable {
            let _ = classes.delete_subkey_all(&id.prog_id);
            for ext in MEDIA_EXTENSIONS {
                if let Ok(k) = classes.open_subkey_with_flags(format!(".{ext}\\OpenWithProgids"), winreg::enums::KEY_SET_VALUE) {
                    let _ = k.delete_value(&id.prog_id);
                }
            }
            let _ = hkcu.delete_subkey_all(format!("Software\\{}", id.app_key));
            if let Ok(k) = hkcu.open_subkey_with_flags("Software\\RegisteredApplications", winreg::enums::KEY_SET_VALUE) {
                let _ = k.delete_value(&id.app_name);
            }
            return Ok(());
        }
        let exe = exe.to_string_lossy();
        let (prog, _) = classes.create_subkey(&id.prog_id)?;
        prog.set_value("", &format!("{} mediabestand", id.app_name))?;
        let (cmd, _) = prog.create_subkey("shell\\open\\command")?;
        cmd.set_value("", &format!("\"{exe}\" \"%1\""))?;
        let (icon, _) = prog.create_subkey("DefaultIcon")?;
        icon.set_value("", &format!("\"{exe}\",0"))?;

        let (caps, _) = hkcu.create_subkey(format!("Software\\{}\\Capabilities", id.app_key))?;
        caps.set_value("ApplicationName", &id.app_name)?;
        caps.set_value("ApplicationDescription", &"Mediaspeler met bibliotheek, Audio Studio, Video Studio en converter")?;
        let (assoc, _) = caps.create_subkey("FileAssociations")?;
        for ext in MEDIA_EXTENSIONS {
            assoc.set_value(format!(".{ext}"), &id.prog_id)?;
            let (k, _) = classes.create_subkey(format!(".{ext}\\OpenWithProgids"))?;
            k.set_raw_value(&id.prog_id, &winreg::RegValue { bytes: std::borrow::Cow::Borrowed(&[]), vtype: winreg::enums::REG_NONE })?;
        }
        let (reg, _) = hkcu.create_subkey("Software\\RegisteredApplications")?;
        reg.set_value(&id.app_name, &format!("Software\\{}\\Capabilities", id.app_key))?;
        Ok(())
    }

    pub fn set_context_menu(id: &Identity, exe: &Path, enable: bool) -> io::Result<()> {
        let classes = classes()?;
        for ext in MEDIA_EXTENSIONS {
            let base = format!("SystemFileAssociations\\.{ext}\\shell");
            if !enable {
                if let Ok(shell) = classes.open_subkey_with_flags(&base, winreg::enums::KEY_ALL_ACCESS) {
                    let _ = shell.delete_subkey_all(&id.verb);
                    let _ = shell.delete_subkey_all(format!("{}.Enqueue", id.verb));
                }
                continue;
            }
            let exe = exe.to_string_lossy();
            let (play, _) = classes.create_subkey(format!("{base}\\{}", id.verb))?;
            play.set_value("MUIVerb", &format!("Afspelen met {}", id.app_name))?;
            play.set_value("Icon", &format!("\"{exe}\",0"))?;
            let (c, _) = play.create_subkey("command")?;
            c.set_value("", &format!("\"{exe}\" \"%1\""))?;
            let (enq, _) = classes.create_subkey(format!("{base}\\{}.Enqueue", id.verb))?;
            enq.set_value("MUIVerb", &"Toevoegen aan wachtrij")?;
            enq.set_value("Icon", &format!("\"{exe}\",0"))?;
            let (c, _) = enq.create_subkey("command")?;
            c.set_value("", &format!("\"{exe}\" --enqueue \"%1\""))?;
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn test_identity() -> Identity {
            Identity {
                prog_id: "VLCUltimateTest.Media".into(),
                app_key: "VLCUltimateTest".into(),
                app_name: "VLC Ultimate Test".into(),
                verb: "VLCUltimateTest.Play".into(),
            }
        }

        #[test]
        fn open_with_registration_roundtrips_and_leaves_nothing_behind() {
            let id = test_identity();
            let exe = Path::new("C:\\Program Files\\VLC Ultimate\\vlc-ultimate.exe");
            assert!(!status(&id).open_with_registered);
            set_open_with(&id, exe, true).unwrap();
            assert!(status(&id).open_with_registered);
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let cmd: String =
                hkcu.open_subkey(format!("Software\\Classes\\{}\\shell\\open\\command", id.prog_id)).unwrap().get_value("").unwrap();
            assert_eq!(cmd, "\"C:\\Program Files\\VLC Ultimate\\vlc-ultimate.exe\" \"%1\"");
            set_open_with(&id, exe, false).unwrap();
            assert!(!status(&id).open_with_registered);
            assert!(hkcu.open_subkey(format!("Software\\Classes\\{}", id.prog_id)).is_err());
            assert!(hkcu.open_subkey(format!("Software\\{}", id.app_key)).is_err());
        }

        #[test]
        fn context_menu_roundtrips_and_is_removed_completely() {
            let id = test_identity();
            let exe = Path::new("C:\\app\\vlc-ultimate.exe");
            set_context_menu(&id, exe, true).unwrap();
            assert!(status(&id).context_menu_registered);
            set_context_menu(&id, exe, false).unwrap();
            assert!(!status(&id).context_menu_registered);
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            assert!(hkcu.open_subkey(format!("Software\\Classes\\SystemFileAssociations\\.mkv\\shell\\{}.Enqueue", id.verb)).is_err());
        }
    }
}

#[cfg(windows)]
#[allow(unused_imports)]
pub use imp::*;

#[cfg(not(windows))]
pub fn status(_id: &Identity) -> IntegrationStatus {
    IntegrationStatus { supported: false, open_with_registered: false, context_menu_registered: false }
}
#[cfg(not(windows))]
pub fn set_open_with(_id: &Identity, _exe: &std::path::Path, _enable: bool) -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Alleen beschikbaar op Windows"))
}
#[cfg(not(windows))]
pub fn set_context_menu(_id: &Identity, _exe: &std::path::Path, _enable: bool) -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Alleen beschikbaar op Windows"))
}

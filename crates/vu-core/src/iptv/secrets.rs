//! Opslag voor inloggegevens van streams, los van de database, logs en exports.
//! Op Windows is dat de Credential Manager; elders (en in tests) een tijdelijk geheugen.

use crate::error::{CoreError, Result};
use std::collections::HashMap;
use std::sync::Mutex;

/// Eén geheim (bijvoorbeeld "gebruiker:wachtwoord" of een token) per sleutel.
pub trait SecretStore: Send + Sync {
    fn set(&self, key: &str, value: &str) -> Result<()>;
    fn get(&self, key: &str) -> Result<Option<String>>;
    /// Verwijderen van een onbekende sleutel is geen fout.
    fn delete(&self, key: &str) -> Result<()>;
}

fn check_key(key: &str) -> Result<()> {
    if key.is_empty() || key.len() > 200 || key.chars().any(char::is_control) {
        return Err(CoreError::Invalid("ongeldige sleutel voor een geheim".into()));
    }
    Ok(())
}

/// Geheugenopslag: verdwijnt bij afsluiten. Voor tests en voor systemen zonder Credential Manager.
#[derive(Default)]
pub struct MemorySecretStore(Mutex<HashMap<String, String>>);

impl SecretStore for MemorySecretStore {
    fn set(&self, key: &str, value: &str) -> Result<()> {
        check_key(key)?;
        self.0.lock().unwrap().insert(key.to_string(), value.to_string());
        Ok(())
    }
    fn get(&self, key: &str) -> Result<Option<String>> {
        check_key(key)?;
        Ok(self.0.lock().unwrap().get(key).cloned())
    }
    fn delete(&self, key: &str) -> Result<()> {
        check_key(key)?;
        self.0.lock().unwrap().remove(key);
        Ok(())
    }
}

/// De standaardopslag: Credential Manager op Windows, anders geheugen.
pub fn default_store() -> Box<dyn SecretStore> {
    #[cfg(windows)]
    {
        Box::new(windows_store::WindowsCredentialStore)
    }
    #[cfg(not(windows))]
    {
        Box::new(MemorySecretStore::default())
    }
}

/// Windows Credential Manager (generieke referenties, per gebruiker bewaard).
#[cfg(windows)]
pub mod windows_store {
    use super::{check_key, CoreError, Result, SecretStore};
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND, FILETIME};
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };

    /// Grens van Windows voor één referentie (CRED_MAX_CREDENTIAL_BLOB_SIZE).
    const MAX_BLOB: usize = 2560;

    pub struct WindowsCredentialStore;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn target(key: &str) -> Vec<u16> {
        wide(&format!("VLC Ultimate/{key}"))
    }

    fn last_error() -> CoreError {
        CoreError::Io(std::io::Error::from_raw_os_error(unsafe { GetLastError() } as i32))
    }

    impl SecretStore for WindowsCredentialStore {
        fn set(&self, key: &str, value: &str) -> Result<()> {
            check_key(key)?;
            let bytes = value.as_bytes();
            if bytes.len() > MAX_BLOB {
                return Err(CoreError::Invalid("het geheim is te lang voor de Credential Manager".into()));
            }
            let mut name = target(key);
            let mut user = wide("vlc-ultimate");
            let cred = CREDENTIALW {
                Flags: 0,
                Type: CRED_TYPE_GENERIC,
                TargetName: name.as_mut_ptr(),
                Comment: null_mut(),
                LastWritten: FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 },
                CredentialBlobSize: bytes.len() as u32,
                CredentialBlob: bytes.as_ptr() as *mut u8,
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                AttributeCount: 0,
                Attributes: null_mut(),
                TargetAlias: null_mut(),
                UserName: user.as_mut_ptr(),
            };
            // SAFETY: `cred` verwijst naar buffers die tot na de aanroep leven; Windows kopieert de gegevens.
            if unsafe { CredWriteW(&cred, 0) } == 0 {
                return Err(last_error());
            }
            Ok(())
        }

        fn get(&self, key: &str) -> Result<Option<String>> {
            check_key(key)?;
            let name = target(key);
            let mut found: *mut CREDENTIALW = null_mut();
            // SAFETY: `name` is nul-afgesloten; bij succes bezit Windows `found` tot CredFree.
            if unsafe { CredReadW(name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut found) } == 0 {
                return if unsafe { GetLastError() } == ERROR_NOT_FOUND { Ok(None) } else { Err(last_error()) };
            }
            // SAFETY: `found` is geldig tot CredFree; blob en grootte horen bij elkaar.
            let text = unsafe {
                let c = &*found;
                let value = if c.CredentialBlob.is_null() || c.CredentialBlobSize == 0 {
                    String::new()
                } else {
                    String::from_utf8_lossy(std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize)).into_owned()
                };
                CredFree(found as *const _);
                value
            };
            Ok(Some(text))
        }

        fn delete(&self, key: &str) -> Result<()> {
            check_key(key)?;
            let name = target(key);
            // SAFETY: `name` is nul-afgesloten.
            if unsafe { CredDeleteW(name.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0 && unsafe { GetLastError() } != ERROR_NOT_FOUND {
                return Err(last_error());
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_real_credential_manager_stores_reads_and_deletes_a_secret() {
            let unique = format!(
                "test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
            );
            let s = WindowsCredentialStore;
            assert_eq!(s.get(&unique).unwrap(), None);
            s.set(&unique, "jan:wachtwoord-é€").unwrap();
            assert_eq!(s.get(&unique).unwrap().as_deref(), Some("jan:wachtwoord-é€"));
            s.set(&unique, "ander").unwrap();
            assert_eq!(s.get(&unique).unwrap().as_deref(), Some("ander"));
            s.delete(&unique).unwrap();
            assert_eq!(s.get(&unique).unwrap(), None);
            s.delete(&unique).unwrap();
        }

        #[test]
        fn too_long_secrets_are_refused() {
            assert!(WindowsCredentialStore.set("test-lang", &"x".repeat(MAX_BLOB + 1)).is_err());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_can_be_stored_read_replaced_and_deleted() {
        let s = MemorySecretStore::default();
        assert_eq!(s.get("bron-1").unwrap(), None);
        s.set("bron-1", "jan:pw").unwrap();
        assert_eq!(s.get("bron-1").unwrap().as_deref(), Some("jan:pw"));
        s.set("bron-1", "jan:nieuw").unwrap();
        assert_eq!(s.get("bron-1").unwrap().as_deref(), Some("jan:nieuw"));
        s.delete("bron-1").unwrap();
        assert_eq!(s.get("bron-1").unwrap(), None);
        s.delete("bron-1").unwrap(); // dubbel verwijderen mag
    }

    #[test]
    fn keys_are_validated() {
        let s = MemorySecretStore::default();
        assert!(s.set("", "x").is_err());
        assert!(s.set("a\nb", "x").is_err());
        assert!(s.get(&"k".repeat(201)).is_err());
    }
}

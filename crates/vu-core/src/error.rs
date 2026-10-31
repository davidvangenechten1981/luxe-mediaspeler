use serde::Serialize;

/// Eén foutsoort voor de hele kern; gaat als tekst naar de interface.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Ongeldige invoer: {0}")]
    Invalid(String),
    #[error("Niet gevonden: {0}")]
    NotFound(String),
    #[error("Bestandsfout: {0}")]
    Io(#[from] std::io::Error),
    #[error("Databasefout: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Media kon niet worden geanalyseerd: {0}")]
    Probe(String),
    #[error("FFmpeg-fout: {0}")]
    Ffmpeg(String),
    #[error("Afspeelengine: {0}")]
    Engine(String),
    #[error("Geannuleerd")]
    Cancelled,
}

impl Serialize for CoreError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;

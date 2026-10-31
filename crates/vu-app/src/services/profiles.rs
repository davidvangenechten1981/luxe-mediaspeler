//! Gebruikersprofielen: eigen kijkstatus en geschiedenis per persoon, gedeelde bibliotheek.

use crate::state::AppState;
use serde::Serialize;
use vu_core::db::Profile;
use vu_core::error::Result;

#[derive(Debug, Serialize)]
pub struct Profiles {
    pub profiles: Vec<Profile>,
    pub active: i64,
}

pub fn list(st: &AppState) -> Result<Profiles> {
    let db = st.lock_db()?;
    Ok(Profiles { profiles: db.list_profiles()?, active: db.profile() })
}

pub fn create(st: &AppState, name: &str) -> Result<i64> {
    st.lock_db()?.create_profile(name)
}

pub fn switch(st: &AppState, id: i64) -> Result<()> {
    st.lock_db()?.set_profile(id)
}

pub fn delete(st: &AppState, id: i64) -> Result<()> {
    st.lock_db()?.delete_profile(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::app;

    #[test]
    fn create_switch_and_delete_a_profile_through_the_service() {
        let app = app();
        let id = create(&app.state, "Kinderen").unwrap();
        switch(&app.state, id).unwrap();
        let l = list(&app.state).unwrap();
        assert_eq!(l.active, id);
        assert_eq!(l.profiles.len(), 2);
        assert!(delete(&app.state, id).is_err());
        switch(&app.state, 1).unwrap();
        delete(&app.state, id).unwrap();
        assert_eq!(list(&app.state).unwrap().profiles.len(), 1);
    }
}

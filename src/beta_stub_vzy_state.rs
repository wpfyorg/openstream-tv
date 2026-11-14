//! Placeholder for the beta VZY provider state in stable builds.
//!
//! The beta providers are compiled out of stable releases; this empty type
//! keeps `AppState`'s layout (and the test fixtures that construct it) valid
//! without pulling in the provider client. No method is reachable from a
//! stable build because every beta code path is feature-gated.

#[derive(Debug, Default)]
pub struct VzyState;

impl VzyState {
    pub fn new(_enabled: bool) -> Self {
        Self
    }
}

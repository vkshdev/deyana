pub mod affective;
pub mod fact_extractor;
pub mod modulation;
pub mod proactive;
pub mod reflection;

use std::sync::{Arc, RwLock};
pub use affective::{AffectiveState, AffectiveStateMachine, DiscreteEmotion};
pub use modulation::{format_self_model_directives, generate_behavioral_directive};

#[derive(Clone)]
pub struct CognitionState {
    pub affective: Arc<RwLock<AffectiveStateMachine>>,
    pub pending_correction: Arc<RwLock<Option<String>>>,
}

impl Default for CognitionState {
    fn default() -> Self {
        Self {
            affective: Arc::new(RwLock::new(AffectiveStateMachine::new())),
            pending_correction: Arc::new(RwLock::new(None)),
        }
    }
}

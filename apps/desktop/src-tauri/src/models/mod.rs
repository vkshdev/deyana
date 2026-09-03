pub mod dataset;
pub mod downloader;
pub mod hardware;
pub mod llamacpp;
pub mod ollama;
pub mod registry;
pub mod router;
pub mod types;

use std::sync::Arc;

pub use downloader::HfDownloader;
pub use hardware::HardwareProfileDetector;
pub use registry::{slugify, GgufRegistry, GgufRegistryState};
pub use router::LlmRouter;
pub use types::*;








#[derive(Clone)]
pub struct LlmState {
    pub router: Arc<LlmRouter>,
}

impl Default for LlmState {
    fn default() -> Self {
        Self {
            router: Arc::new(LlmRouter::default()),
        }
    }
}

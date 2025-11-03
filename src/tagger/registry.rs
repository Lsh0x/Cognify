use crate::tagger::{generic::GenericHandler, text::MarkdownHandler, text::TextHandler, Taggable};
use std::sync::Arc;

/// Registry for managing file type handlers
pub struct TaggerRegistry {
    handlers: Vec<Arc<dyn Taggable>>,
    generic: Arc<dyn Taggable>,
}

impl TaggerRegistry {
    /// Create a new registry with default handlers
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: Vec::new(),
            generic: Arc::new(GenericHandler::new()),
        };

        // Register default handlers
        registry.register(Arc::new(TextHandler::new()));
        registry.register(Arc::new(MarkdownHandler::new()));

        registry
    }

    /// Register a new handler
    pub fn register(&mut self, handler: Arc<dyn Taggable>) {
        self.handlers.push(handler);
    }

    /// Find a handler that supports the given extension
    pub fn get_handler(&self, ext: &str) -> Arc<dyn Taggable> {
        for handler in &self.handlers {
            if handler.supports_extension(ext) {
                return handler.clone();
            }
        }

        // Fallback to generic handler
        self.generic.clone()
    }
}

impl Default for TaggerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_handles_txt() {
        let registry = TaggerRegistry::new();
        let handler = registry.get_handler("txt");
        assert!(handler.supports_extension("txt"));
    }

    #[test]
    fn test_registry_handles_md() {
        let registry = TaggerRegistry::new();
        let handler = registry.get_handler("md");
        assert!(handler.supports_extension("md"));
    }

    #[test]
    fn test_registry_fallback_to_generic() {
        let registry = TaggerRegistry::new();
        let handler = registry.get_handler("unknown");
        assert!(handler.supports_extension("unknown"));
    }
}


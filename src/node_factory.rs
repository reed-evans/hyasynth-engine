// src/node_factory.rs

use crate::node::{Node, Polyphony};

/// A factory capable of creating fresh node instances.
///
/// This is only used during graph construction / preparation.
pub trait NodeFactory: Send {
    /// Create one node instance
    fn create(&self) -> Box<dyn Node>;

    /// Polyphony behavior of nodes created by this factory
    fn polyphony(&self) -> Polyphony;

    /// Number of output channels this node produces
    fn num_channels(&self) -> usize;
}

/// Convenience factory for simple nodes
pub struct SimpleNodeFactory<F>
where
    F: Fn() -> Box<dyn Node> + Send,
{
    create_fn: F,
    polyphony: Polyphony,
}

impl<F> SimpleNodeFactory<F>
where
    F: Fn() -> Box<dyn Node> + Send,
{
    pub fn new(create_fn: F, polyphony: Polyphony) -> Self {
        Self {
            create_fn,
            polyphony,
        }
    }
}

impl<F> NodeFactory for SimpleNodeFactory<F>
where
    F: Fn() -> Box<dyn Node> + Send,
{
    fn create(&self) -> Box<dyn Node> {
        (self.create_fn)()
    }

    fn polyphony(&self) -> Polyphony {
        self.polyphony
    }

    fn num_channels(&self) -> usize {
        2
    }
}

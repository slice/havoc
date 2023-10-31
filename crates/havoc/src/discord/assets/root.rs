use std::fmt::Display;

/// The types of various `<script>` tags in Discord application's HTML.
/// Keep in mind that these are fragile assumptions and could potentially
/// change at any time.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RootScript {
    /// A script which handles the loading of other Webpack chunks that aren't
    /// present at the root.
    ChunkLoader,

    /// The Webpack chunk containing CSS chunk class mappings.
    Classes,

    /// The Webpack chunk containing various vendor modules, such as Sentry.
    Vendor,

    /// The principal Webpack chunk containing the bulk of the app code.
    Entrypoint,
}

impl Display for RootScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use RootScript::*;

        match self {
            ChunkLoader => write!(f, "chunk loader"),
            Classes => write!(f, "classes"),
            Vendor => write!(f, "vendor"),
            Entrypoint => write!(f, "entrypoint"),
        }
    }
}

impl RootScript {
    /// Given a number of script tags present in the HTML of a frontend, returns
    /// the assumed index of the script corresponding to this `RootScript`.
    pub fn assumed_index_within_scripts(&self, n_scripts: usize) -> Option<usize> {
        use RootScript::*;

        match self {
            // Seemingly always last.
            ChunkLoader => n_scripts.checked_sub(1),
            // Seemingly always first.
            Classes => Some(0),
            // Seemingly always penultimate. Nota bene: it's now no longer clear
            // to me if the concept of an "entrypoint" still applies with
            // Rspack. Anyhow, it's a bit of a vague term, so this needs further
            // design.
            Entrypoint => n_scripts.checked_sub(2),
            // In an Rspack world, it doesn't make sense to pinpoint a specific
            // index for this.
            Vendor => None,
        }
    }
}

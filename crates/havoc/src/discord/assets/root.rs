/// The types of various `<script>` tags in Discord application's HTML.
/// Keep in mind that these are fragile assumptions and could potentially
/// change at any time.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RootScript {
    /// A script which handles loading other Webpack chunks that aren't root
    /// script.
    ChunkLoader,

    /// The Webpack chunk containing CSS chunk class mappings.
    Classes,

    /// The Webpack chunk containing various vendor modules, such as Sentry.
    Vendor,

    /// The principal Webpack chunk containing the bulk of the app code.
    Entrypoint,
}

impl RootScript {
    /// Returns the assumed ordering of the root scripts in the application HTML.
    ///
    /// This is a fragile assumption that could change at any time.
    pub fn assumed_ordering() -> [RootScript; 4] {
        use RootScript::*;

        [ChunkLoader, Classes, Vendor, Entrypoint]
    }

    /// Using the assumed ordering of the root scripts in the application HTML,
    /// returns the index into that ordering for this root script.
    ///
    /// This is a fragile assumption that could change at any time.
    pub fn assumed_index(&self) -> usize {
        Self::assumed_ordering()
            .iter()
            .position(|kind| kind == self)
            .expect(
                "invariant violation: RootScript::assumed_ordering doesn't contain all variants",
            )
    }
}

//! Something that you can dump from.

use std::fmt::Display;

use crate::discord::Assets;

/// Something that you can dump information from.
///
/// It is assumed that artifacts contain associated [`FeAsset`](crate::discord::FeAsset)s,
/// accessible by calling [`assets`](Artifact::assets). The [`dump_prefix`](Artifact::dump_prefix)
/// is used when writing out dumped data to disk.
///
/// This trait does not provide any facilities for dumping useful data itself; instead,
/// things that implement this trait are "consumed" by the
/// [`Dump::dump`](crate::dump::Dump::dump) method.
pub trait Artifact: Display {
    /// Returns the dump prefix for this artifact, which is intended to be
    /// prepended to [`DumpResult`](crate::dump::DumpResult) filenames when dumping.
    fn dump_prefix(&self) -> String {
        "".to_owned()
    }

    /// Returns the assets associated with this artifact.
    fn assets(&self) -> &Assets;
}

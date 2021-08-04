//! Something that you can dump from.

use std::error::Error;
use std::fmt::Display;
use std::rc::Rc;

use crate::discord::{Assets, FeAsset};
use crate::dump::{DumpItem, DumpResult};

/// Something that you can dump information from. It is also assumed that
/// artifacts have some associated [`FeAsset`](crate::discord::FeAsset)s,
/// accessible through [`assets`](Artifact::assets).
///
/// This provides a shared interface between things that you can extract data
/// from. This is principally used by the command line application, where you
/// specify some artifact to be scraped, and what to dump from it. If you have
/// a specific goal in mind, it may be best to use the types directly instead
/// of the `Artifact` abstraction.
///
/// Check whether a dump item is supported with
/// [`supports_dump_item`](Artifact::supports_dump_item), and
/// dump that item with the [`dump`](Artifact::dump) method.
pub trait Artifact: Display {
    /// Returns whether a particular dump item is supported or not.
    fn supports_dump_item(&self, _item: DumpItem) -> bool {
        false
    }

    /// Returns the dump prefix for this artifact, which is intended to be
    /// prepended to [`DumpResult`] filenames when dumping.
    fn dump_prefix(&self) -> String {
        "".to_owned()
    }

    /// Dumps some data from this artifact.
    fn dump(
        &self,
        item: DumpItem,
        assets: &mut Assets,
    ) -> Result<Vec<DumpResult>, Box<dyn Error + Send + Sync>>;

    /// Returns the assets associated with this artifact.
    fn assets(&self) -> &[Rc<FeAsset>];
}

use std::error::Error;
use std::fmt::Display;
use std::rc::Rc;

use crate::discord::{Assets, FeAsset};
use crate::dump::{DumpItem, DumpResult};

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

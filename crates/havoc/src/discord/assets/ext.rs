use crate::discord::{FeAsset, FeAssetType, RootScript};

pub trait AssetsExt<'a, I> {
    // We can't use "return position impl trait in traits" here, so we're forced
    // to either box or implement our own iterator type. I can't be bothered to
    // write a new iterator type at the moment so we'll just box.
    fn filter_by_type(self, typ: FeAssetType) -> Box<dyn Iterator<Item = &'a FeAsset> + 'a + Send>;

    fn find_root_script(self, root_script_type: RootScript) -> Option<&'a FeAsset>
    where
        Self: Sized,
    {
        self.filter_by_type(FeAssetType::Js)
            .nth(root_script_type.assumed_index())
    }
}

impl<'a, I: Iterator<Item = &'a FeAsset> + 'a + Send> AssetsExt<'a, I> for I {
    fn filter_by_type(self, typ: FeAssetType) -> Box<dyn Iterator<Item = &'a FeAsset> + 'a + Send> {
        Box::new(self.filter(move |asset| asset.typ == typ))
    }
}

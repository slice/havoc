use crate::discord::{FeAsset, FeAssetType, RootScript};

pub trait AssetsExt<'source, Source: ?Sized> {
    // We can't use "return position impl trait in traits" here, so we're forced
    // to either box or implement our own iterator type. I can't be bothered to
    // write a new iterator type at the moment so we'll just box.
    fn filter_by_type(
        self,
        typ: FeAssetType,
    ) -> Box<dyn Iterator<Item = &'source FeAsset> + 'source + Send>;

    fn find_root_script(self, root_script_type: RootScript) -> Option<&'source FeAsset>
    where
        Self: Sized,
    {
        let scripts = self.filter_by_type(FeAssetType::Js).collect::<Vec<_>>();

        root_script_type
            .assumed_index_within_scripts(scripts.len())
            .and_then(|index| scripts.into_iter().nth(index))
    }
}

// Implement the convenience extension on any reference to a source, where said
// reference can be turned into an iterator yielding references to assets within
// the source. The source doesn't have to be sized, so we can use slices with
// it, which are notably unsized when not behind some indirection (e.g. a
// reference).
impl<'source, Source: ?Sized> AssetsExt<'source, Source> for &'source Source
where
    &'source Source: IntoIterator<Item = &'source FeAsset> + Send,
    <&'source Source as IntoIterator>::IntoIter: Send + 'source,
{
    fn filter_by_type(
        self,
        typ: FeAssetType,
    ) -> Box<dyn Iterator<Item = &'source FeAsset> + 'source + Send> {
        Box::new(self.into_iter().filter(move |asset| asset.typ == typ))
    }
}

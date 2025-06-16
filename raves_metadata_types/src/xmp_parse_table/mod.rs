use std::sync::LazyLock;

use rustc_hash::FxHashMap;

/// A pair of a property's namespace URL and its element name.
///
/// Ex: ("http://ns.adobe.com/xap/1.0/", "CreateDate") for `xmp:CreateDate`
pub type XmpNamespaceNamePair = (&'static str, &'static str);

/// A map, (key, value), where:
///
/// - `key` is the namespace URL + name pair
/// - `value` is some recursive data structure representing how to parse its
///   matching key.
pub static XMP_PARSING_MAP: LazyLock<FxHashMap<XmpNamespaceNamePair, Kind>> = LazyLock::new(|| {
    let mut m: FxHashMap<XmpNamespaceNamePair, Kind> = FxHashMap::default();
    map(&mut m);
    m
});

/// Adds all (key, value) pairs to the currently empty map.
fn map(m: &mut FxHashMap<XmpNamespaceNamePair, Kind>) {}

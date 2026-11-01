//! The .rgr classes the editor crate's modules use, imported once.

#[allow(dead_code, unused, non_snake_case)]
mod rgr {
    ranger::import_rgr!("../../../../lib/evg/EVGA11yTree");
    ranger::import_rgr!("../../../../lib/evg/EVGImageDecode");
    ranger::import_rgr!("../../../../lib/evg/EVGLayout");
    ranger::import_rgr!("../../../../lib/evg/EVGStyleSheet");
    ranger::import_rgr!("../../../../lib/evg/EVGToolbar");
    ranger::import_rgr!("../../../../lib/evg/PathBuilder");
    ranger::import_rgr!("../../../../lib/image/ImageBuffer");
    ranger::import_rgr!("../../../css/CssCore");
    ranger::import_rgr!("../../../game_engine/framebuffer");
    ranger::import_rgr!("../../../game_engine/ui/UIContext");
    ranger::import_rgr!("../../../game_engine/ui/UITextRenderer");
    ranger::import_rgr!("../../../office/assets/OfficeAsset");
    ranger::import_rgr!("../../../office/editor/OfficeHistory");
    ranger::import_rgr!("../../../office/text/OfficeCaretMotion");
    ranger::import_rgr!("../../../vela/src/VlShape");
    ranger::import_rgr!("../../../vfs/src/Vfs");
    ranger::import_rgr!("../../../vfs/src/VfsCore");
    ranger::import_rgr!("../../../zip/ZipWriter");
}
pub use rgr::{CssCascade, CssComputed, CssNode, CssSheet, CssValue, EVGA11yNode, EVGA11yRole, EVGA11yTree, EVGImageDecode, EVGLayout, EVGStyleSheet, EVGToolbar, ImageBuffer, OfficeAssetStore, OfficeCaretMotion, PathBuilder, PathCommand, SoftCanvas, ToolItem, ToolTab, UIContext, UITextRenderer, Vfs, VfsStat, VlShape, ZipWriter};

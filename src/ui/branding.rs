//! Embedded brand artwork: a transparent sidebar mark and a white application icon.
use gpui_kit::*;
use std::sync::{Arc, LazyLock};

pub(crate) const COPYRIGHT: &str = "Copyright © Moenya";

pub(crate) fn sidebar_logo() -> Img {
    static LOGO: LazyLock<Arc<Image>> = LazyLock::new(|| {
        Arc::new(Image::from_bytes(
            ImageFormat::Svg,
            include_bytes!("../../assets/logo-mark.svg").to_vec(),
        ))
    });
    img(LOGO.clone())
}

pub(crate) fn logo() -> Img {
    static LOGO: LazyLock<Arc<Image>> = LazyLock::new(|| {
        Arc::new(Image::from_bytes(
            ImageFormat::Svg,
            include_bytes!("../../assets/logo.svg").to_vec(),
        ))
    });
    img(LOGO.clone())
}

use raves_metadata::magic_number::{AnyProvider, MagicNumber};

fn logger() {
    _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::max())
        .format_file(true)
        .format_line_number(true)
        .try_init();
}

/// Checks that `raves_metadata::get` works as expected!
#[test]
fn _get_and_parse() {
    logger();

    let paths = &[
        ("assets/providers/avif/bbb_4k.avif", MagicNumber::Avif),
        ("assets/providers/heic/C034.heic", MagicNumber::Heif),
        (
            "assets/providers/jpeg/Calico_Cat_Asleep.jpg",
            MagicNumber::Jpeg,
        ),
        ("assets/providers/mov/QuickTime.mov", MagicNumber::Mov),
        ("assets/providers/png/exif.png", MagicNumber::Png),
        ("assets/providers/webp/RIFF.webp", MagicNumber::Webp),
        ("assets/01_simple_with_aves_tags.mp4", MagicNumber::Mp4),
    ];

    for (ref path, ty) in *paths {
        log::debug!("TEST: {ty:?} for path: `{path}`");

        let v: Vec<u8> = std::fs::read(path).expect("given path should be valid");
        let file: &[u8] = v.as_slice();

        // try `raves_metadata::get`
        let got: Option<MagicNumber> = raves_metadata::get(&file);
        assert_eq!(got, Some(ty), "`get` should find matching type");

        // and `raves_metadata::parse`
        let parsed: Option<AnyProvider> = raves_metadata::parse(&file);

        // ensure
        assert_eq!(
            Into::<MagicNumber>::into(
                parsed.expect("magic number passed, so file should parse successfully")
            ),
            ty,
            "`parse` should successfully parse to matching type"
        );

        log::debug!("END TEST {ty:?} (success!)\n\n");
    }
}

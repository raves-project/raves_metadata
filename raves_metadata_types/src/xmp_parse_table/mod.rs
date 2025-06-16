use std::sync::LazyLock;

use rustc_hash::FxHashMap;

use crate::xmp_parsing_types::{XmpKind as Kind, XmpPrimitiveKind as Prim};

pub mod types;

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
fn map(m: &mut FxHashMap<XmpNamespaceNamePair, Kind>) {
    // helper lambda to make things slightly shorter :D
    let mut i = |key: XmpNamespaceNamePair, value: Kind| m.insert(key, value);

    //
    //
    //
    //
    //
    // BEGIN XMP STANDARD NAMESPACES
    //
    // these are pretty common - they're expected to be generally useful.
    //

    /*
     * Adobe XMP Basic namespace
     *
     * "The XMP basic namespace contains properties that provide basic
     * descriptive information."
     */
    {
        const BASIC: &str = "http://ns.adobe.com/xap/1.0/";
        i((BASIC, "CreateDate"), Kind::Simple(Prim::Date));
        i((BASIC, "CreatorTool"), types::AGENT_NAME);
        i(
            (BASIC, "Identifier"),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
        i((BASIC, "Label"), Kind::Simple(Prim::Text));
        i((BASIC, "MetadataDate"), Kind::Simple(Prim::Date));
        i((BASIC, "ModifyDate"), Kind::Simple(Prim::Date));
        i((BASIC, "Rating"), Kind::Simple(Prim::Real));
        i((BASIC, "BaseURL"), types::URL);
        i((BASIC, "Nickname"), Kind::Simple(Prim::Text));
        i((BASIC, "Thumbnails"), types::THUMBNAIL);
    }

    /*
     * XMP Media Management namespace
     *
     * "This namespace is primarily for use by digital asset management (DAM)
     * systems."
     */
    {
        const XMP_MM: &str = "http://ns.adobe.com/xap/1.0/mm/";
        i((XMP_MM, "DerivedFrom"), types::RESOURCE_REF);
        i((XMP_MM, "DocumentID"), types::GUID);
        i((XMP_MM, "InstanceID"), types::GUID);
        i((XMP_MM, "OriginalDocumentID"), types::GUID);
        i((XMP_MM, "RenditionClass"), types::RENDITION_CLASS);
        i((XMP_MM, "RenditionParams"), Kind::Simple(Prim::Text));
        i(
            (XMP_MM, "History"),
            Kind::OrderedArray(&types::RESOURCE_EVENT),
        );
        i(
            (XMP_MM, "Ingredients"),
            Kind::UnorderedArray(&types::RESOURCE_REF),
        );
        i(
            (XMP_MM, "Pantry"),
            Kind::UnorderedArray(&Kind::StructUnspecifiedFields {
                // TODO: make it such that you can add requirements for each struct
                // in the array. why? well...
                //
                // technically, "each pantry entry" must have an `xmpMM:InstanceID`
                // field, but checking this would require the compiler to know that
                // each entry in the list is a Kind::Struct, which we can't prove
                // statically.
                //
                // GUID is just a text field, so we can leave it unspecified for
                // now and come back later.
                required_fields: &[],
            }),
        );
        i(
            (XMP_MM, "ManagedFrom"),
            Kind::UnorderedArray(&types::RESOURCE_REF),
        );
        i((XMP_MM, "Manager"), types::AGENT_NAME);
        i((XMP_MM, "ManageTo"), types::URI);
        i((XMP_MM, "ManageUI"), types::URI);
        i((XMP_MM, "ManagerVariant"), Kind::Simple(Prim::Text));
        i((XMP_MM, "VersionID"), Kind::Simple(Prim::Text));
        i((XMP_MM, "Versions"), Kind::OrderedArray(&types::VERSION));
    }

    /*
     * Basic Job Ticket namespace
     *
     * "This namespace describes very simple workflow or job information."
     */
    {
        // incredible name, props to whoever pulled this off
        const XMP_BJ: &str = "http://ns.adobe.com/xap/1.0/bj/";

        // LOL and it's for one type too
        i((XMP_BJ, "JobRef"), Kind::UnorderedArray(&types::JOB));
    }

    /*
     * XMP Paged-Text namespace
     *
     * "The Paged-Text namespace is used for text appearing on a page in a
     * document."
     */
    {
        const XMP_TPG: &str = "http://ns.adobe.com/xap/1.0/t/pg/";
        i((XMP_TPG, "Colorants"), Kind::OrderedArray(&types::COLORANT));
        i((XMP_TPG, "Fonts"), Kind::UnorderedArray(&types::FONT));
        i((XMP_TPG, "MaxPageSize"), types::DIMENSIONS);
        i((XMP_TPG, "NPages"), Kind::Simple(Prim::Integer));
        i(
            (XMP_TPG, "PlateNames"),
            Kind::OrderedArray(&Kind::Simple(Prim::Text)),
        );
    }

    /*
     * XMP Dynamic Media namespace
     *
     * "This namespace specifies properties used by the Adobe dynamic media
     * group."
     */
    {
        const XMP_DM: &str = "http://ns.adobe.com/xmp/1.0/DynamicMedia/";
        i((XMP_DM, "absPeakAudioFilePath"), types::URI);
        i((XMP_DM, "album"), Kind::Simple(Prim::Text));
        i((XMP_DM, "altTapeName"), Kind::Simple(Prim::Text));
        i((XMP_DM, "altTimecode"), types::TIMECODE);
        i((XMP_DM, "artist"), Kind::Simple(Prim::Text));
        i((XMP_DM, "audioChannelType"), Kind::Simple(Prim::Text));
        i((XMP_DM, "audioCompressor"), Kind::Simple(Prim::Text));
        i((XMP_DM, "audioSampleRate"), Kind::Simple(Prim::Integer));
        i((XMP_DM, "audioSampleType"), Kind::Simple(Prim::Text));
        i((XMP_DM, "beatSpliceParams"), types::BEAT_SPLICE_STRETCH);
        i((XMP_DM, "cameraAngle"), Kind::Simple(Prim::Text));
        i((XMP_DM, "cameraLabel"), Kind::Simple(Prim::Text));
        i((XMP_DM, "cameraModel"), Kind::Simple(Prim::Text));
        i((XMP_DM, "cameraMove"), Kind::Simple(Prim::Text));
        i((XMP_DM, "client"), Kind::Simple(Prim::Text));
        i((XMP_DM, "comment"), Kind::Simple(Prim::Text));
        i((XMP_DM, "composer"), Kind::Simple(Prim::Text));
        i(
            (XMP_DM, "contributedMedia"),
            Kind::UnorderedArray(&types::MEDIA),
        );
        i((XMP_DM, "director"), Kind::Simple(Prim::Text));
        i((XMP_DM, "directorPhotography"), Kind::Simple(Prim::Text));
        i((XMP_DM, "duration"), types::TIME);
        i((XMP_DM, "engineer"), Kind::Simple(Prim::Text));
        i((XMP_DM, "fileDataRate"), types::RATIONAL);
        i((XMP_DM, "genre"), Kind::Simple(Prim::Text));
        i((XMP_DM, "good"), Kind::Simple(Prim::Boolean));
        i((XMP_DM, "instrument"), Kind::Simple(Prim::Text));
        i((XMP_DM, "introTime"), types::TIME);
        i((XMP_DM, "key"), Kind::Simple(Prim::Text));
        i((XMP_DM, "logComment"), Kind::Simple(Prim::Text));
        i((XMP_DM, "loop"), Kind::Simple(Prim::Boolean));
        i((XMP_DM, "numberOfBeats"), Kind::Simple(Prim::Real));
        i((XMP_DM, "markers"), Kind::OrderedArray(&types::MARKER));
        i((XMP_DM, "outCue"), types::TIME);
        i((XMP_DM, "projectName"), Kind::Simple(Prim::Text));
        i((XMP_DM, "projectRef"), types::PROJECT_LINK);
        i((XMP_DM, "pullDown"), Kind::Simple(Prim::Text));
        i((XMP_DM, "relativePeakAudioFilePath"), types::URI);
        i((XMP_DM, "relativeTimestamp"), types::TIME);
        i((XMP_DM, "releaseDate"), Kind::Simple(Prim::Date));
        i((XMP_DM, "resampleParams"), types::RESAMPLE_STRETCH);
        i((XMP_DM, "scaleType"), Kind::Simple(Prim::Text));
        i((XMP_DM, "scene"), Kind::Simple(Prim::Text));
        i((XMP_DM, "shotDate"), Kind::Simple(Prim::Date));
        i((XMP_DM, "shotDay"), Kind::Simple(Prim::Text));
        i((XMP_DM, "shotLocation"), Kind::Simple(Prim::Text));
        i((XMP_DM, "shotName"), Kind::Simple(Prim::Text));
        i((XMP_DM, "shotNumber"), Kind::Simple(Prim::Text));
        i((XMP_DM, "shotSize"), Kind::Simple(Prim::Text));
        i((XMP_DM, "speakerPlacement"), Kind::Simple(Prim::Text));
        i((XMP_DM, "startTimecode"), types::TIMECODE);
        i((XMP_DM, "stretchMode"), Kind::Simple(Prim::Text));
        i((XMP_DM, "takeNumber"), Kind::Simple(Prim::Integer));
        i((XMP_DM, "tapeName"), Kind::Simple(Prim::Text));
        i((XMP_DM, "tempo"), Kind::Simple(Prim::Real));
        i((XMP_DM, "timeScaleParams"), types::TIME_SCALE_STRETCH);
        i((XMP_DM, "timeSignature"), Kind::Simple(Prim::Text));
        i((XMP_DM, "trackNumber"), Kind::Simple(Prim::Integer));
        i((XMP_DM, "Tracks"), Kind::UnorderedArray(&types::TRACK));
        i((XMP_DM, "videoAlphaMode"), Kind::Simple(Prim::Text));
        i((XMP_DM, "videoAlphaPremultipleColor"), types::COLORANT);
        i(
            (XMP_DM, "videoAlphaUnityIsTransparent"),
            Kind::Simple(Prim::Boolean),
        );
        i((XMP_DM, "videoColorSpace"), Kind::Simple(Prim::Text));
        i((XMP_DM, "videoCompressor"), Kind::Simple(Prim::Text));
        i((XMP_DM, "videoFieldOrder"), Kind::Simple(Prim::Text));
        i((XMP_DM, "videoFrameRate"), Kind::Simple(Prim::Text));
        i((XMP_DM, "videoFrameSize"), types::DIMENSIONS);
        i((XMP_DM, "videoPixelAspectRatio"), types::RATIONAL);
        i((XMP_DM, "videoPixelDepth"), Kind::Simple(Prim::Text));
        i((XMP_DM, "partOfCompilation"), Kind::Simple(Prim::Boolean));
        i((XMP_DM, "lyrics"), Kind::Simple(Prim::Text));
        i((XMP_DM, "discNumber"), Kind::Simple(Prim::Text));
    }

    /*
     * XMP Rights Management namespace
     *
     * "The XMP Rights Management namespace contains properties that provide
     * information regarding the legal restrictions associated with a
     * resource."
     */
    {
        const XMP_RIGHTS: &str = "http://ns.adobe.com/xap/1.0/rights/";
        i((XMP_RIGHTS, "Certificate"), Kind::Simple(Prim::Text));
        i((XMP_RIGHTS, "Marked"), Kind::Simple(Prim::Boolean));
        i(
            (XMP_RIGHTS, "Owner"),
            Kind::UnorderedArray(&types::PROPER_NAME),
        );
        i((XMP_RIGHTS, "UsageTerms"), types::LANGUAGE_ALTERNATIVE);
        i((XMP_RIGHTS, "WebStatement"), Kind::Simple(Prim::Text));
    }

    //
    //
    //
    //
    //
    // BEGIN XMP SPECIALIZED NAMESPACES
    //
    // "namespaces... specialized for Adobe applications or usages."
    //

    /*
     * Adobe PDF namespace
     *
     * "This namespace specifies properties used with Adobe PDF documents."
     */
    {
        pub const PDF: &str = "http://ns.adobe.com/pdf/1.3/";
        i((PDF, "Keywords"), Kind::Simple(Prim::Text));
        i((PDF, "PDFVersion"), Kind::Simple(Prim::Text));
        i((PDF, "Producer"), types::AGENT_NAME);

        // "True when the document has been trapped." ???
        //
        // that sounds so scary man D;
        i((PDF, "Trapped"), Kind::Simple(Prim::Boolean));
    }

    /*
     * Photoshop namespace
     *
     * "This namespace specifies properties used by Adobe Photoshop."
     */
    {
        pub const PHOTOSHOP: &str = "http://ns.adobe.com/photoshop/1.0/";

        i((PHOTOSHOP, "ColorMode"), Kind::Simple(Prim::Integer));
        i(
            (PHOTOSHOP, "DocumentAncestors"),
            Kind::UnorderedArray(&types::ANCESTOR),
        );
        i((PHOTOSHOP, "History"), Kind::Simple(Prim::Text));
        i((PHOTOSHOP, "ICCProfile"), Kind::Simple(Prim::Text));
        i((PHOTOSHOP, "TextLayers"), Kind::OrderedArray(&types::LAYER));
        i((PHOTOSHOP, "AuthorsPosition"), Kind::Simple(Prim::Text));
        i((PHOTOSHOP, "CaptionWriter"), types::PROPER_NAME);
        i((PHOTOSHOP, "Category"), Kind::Simple(Prim::Text));
        i((PHOTOSHOP, "City"), Kind::Simple(Prim::Text));
        i((PHOTOSHOP, "Country"), Kind::Simple(Prim::Text));
        i((PHOTOSHOP, "Credit"), Kind::Simple(Prim::Text));
        i((PHOTOSHOP, "DateCreated"), Kind::Simple(Prim::Date));
        i((PHOTOSHOP, "Headline"), Kind::Simple(Prim::Text));
        i((PHOTOSHOP, "Instructions"), Kind::Simple(Prim::Text));
        i((PHOTOSHOP, "Source"), Kind::Simple(Prim::Text));
        i((PHOTOSHOP, "State"), Kind::Simple(Prim::Text));
        i(
            (PHOTOSHOP, "SupplementalCategories"),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
        i(
            (PHOTOSHOP, "TransmissionReference"),
            Kind::Simple(Prim::Text),
        );
        i((PHOTOSHOP, "Urgency"), Kind::Simple(Prim::Integer));
    }

    /*
     * Camera Raw namespace
     *
     * "This namespace specifies settings associated with image files produced
     * in camera raw mode."
     */
    {
        pub const CRS: &str = "http://ns.adobe.com/camera-raw-settings/1.0/";

        i((CRS, "AutoBrightness"), Kind::Simple(Prim::Boolean));
        i((CRS, "AutoContrast"), Kind::Simple(Prim::Boolean));
        i((CRS, "AutoExposure"), Kind::Simple(Prim::Boolean));
        i((CRS, "AutoShadows"), Kind::Simple(Prim::Boolean));
        i((CRS, "BlueHue"), Kind::Simple(Prim::Integer));
        i((CRS, "BlueSaturation"), Kind::Simple(Prim::Integer));
        i((CRS, "Brightness"), Kind::Simple(Prim::Integer));
        i((CRS, "CameraProfile"), Kind::Simple(Prim::Text));
        i((CRS, "ChromaticAberrationB"), Kind::Simple(Prim::Integer));
        i((CRS, "ChromaticAberrationR"), Kind::Simple(Prim::Integer));
        i((CRS, "ColorNoiseReduction"), Kind::Simple(Prim::Integer));
        i((CRS, "Contrast"), Kind::Simple(Prim::Integer));
        i((CRS, "CropTop"), Kind::Simple(Prim::Real));
        i((CRS, "CropLeft"), Kind::Simple(Prim::Real));
        i((CRS, "CropBottom"), Kind::Simple(Prim::Real));
        i((CRS, "CropRight"), Kind::Simple(Prim::Real));
        i((CRS, "CropAngle"), Kind::Simple(Prim::Real));
        i((CRS, "CropWidth"), Kind::Simple(Prim::Real));
        i((CRS, "CropHeight"), Kind::Simple(Prim::Real));
        i((CRS, "CropUnits"), Kind::Simple(Prim::Integer));
        i((CRS, "Exposure"), Kind::Simple(Prim::Real));
        i((CRS, "GreenHue"), Kind::Simple(Prim::Integer));
        i((CRS, "GreenSaturation"), Kind::Simple(Prim::Integer));
        i((CRS, "HasCrop"), Kind::Simple(Prim::Boolean));
        i((CRS, "HasSettings"), Kind::Simple(Prim::Boolean));
        i((CRS, "LuminanceSmoothing"), Kind::Simple(Prim::Integer));
        i((CRS, "RawFileName"), Kind::Simple(Prim::Text));
        i((CRS, "RedHue"), Kind::Simple(Prim::Integer));
        i((CRS, "RedSaturation"), Kind::Simple(Prim::Integer));
        i((CRS, "Saturation"), Kind::Simple(Prim::Integer));
        i((CRS, "Shadows"), Kind::Simple(Prim::Integer));
        i((CRS, "ShadowTint"), Kind::Simple(Prim::Integer));
        i((CRS, "Sharpness"), Kind::Simple(Prim::Integer));
        i((CRS, "Temperature"), Kind::Simple(Prim::Integer));
        i((CRS, "Tint"), Kind::Simple(Prim::Integer));
        i(
            (CRS, "ToneCurve"),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
        i((CRS, "ToneCurveName"), Kind::Simple(Prim::Text));
        i((CRS, "Version"), Kind::Simple(Prim::Text));
        i((CRS, "VignetteAmount"), Kind::Simple(Prim::Integer));
        i((CRS, "VignetteMidpoint"), Kind::Simple(Prim::Integer));
        i((CRS, "WhiteBalance"), Kind::Simple(Prim::Text));
    }

    /*
     * EXIF namespace
     *
     * "EXIF Schema For EXIF-Specific Properties. These properties defined
     * solely by EXIF."
     */
    {
        pub const EXIF: &str = "http://ns.adobe.com/exif/1.0/";

        i((EXIF, "ApertureValue"), types::RATIONAL);
        i((EXIF, "BrightnessValue"), types::RATIONAL);
        i((EXIF, "CFAPattern"), types::CFA_PATTERN);
        i((EXIF, "ColorSpace"), Kind::Simple(Prim::Integer));
        i((EXIF, "CompressedBitsPerPixel"), types::RATIONAL);
        i((EXIF, "Contrast"), Kind::Simple(Prim::Integer));
        i((EXIF, "CustomRendered"), Kind::Simple(Prim::Integer));
        i((EXIF, "DateTimeDigitized"), Kind::Simple(Prim::Date));
        i((EXIF, "DateTimeOriginal"), Kind::Simple(Prim::Date));
        i((EXIF, "DeviceSettingDescription"), types::DEVICE_SETTINGS);
        i((EXIF, "DigitalZoomRatio"), types::RATIONAL);
        i((EXIF, "ExifVersion"), Kind::Simple(Prim::Text));
        i((EXIF, "ExposureBiasValue"), types::RATIONAL);
        i((EXIF, "ExposureIndex"), types::RATIONAL);
        i((EXIF, "ExposureMode"), Kind::Simple(Prim::Integer));
        i((EXIF, "ExposureProgram"), Kind::Simple(Prim::Integer));
        i((EXIF, "ExposureTime"), types::RATIONAL);
        i((EXIF, "FileSource"), Kind::Simple(Prim::Integer));
        i((EXIF, "Flash"), types::FLASH);
        i((EXIF, "FlashEnergy"), types::RATIONAL);
        i((EXIF, "FlashpixVersion"), Kind::Simple(Prim::Text));
        i((EXIF, "FNumber"), types::RATIONAL);
        i((EXIF, "FocalLength"), types::RATIONAL);
        i((EXIF, "FocalLengthIn35mmFilm"), Kind::Simple(Prim::Integer));
        i(
            (EXIF, "FocalPlaneResolutionUnit"),
            Kind::Simple(Prim::Integer),
        );
        i((EXIF, "FocalPlaneXResolution"), types::RATIONAL);
        i((EXIF, "FocalPlaneYResolution"), types::RATIONAL);
        i((EXIF, "GainControl"), Kind::Simple(Prim::Integer));
        i((EXIF, "ImageUniqueID"), Kind::Simple(Prim::Text));
        i(
            (EXIF, "ISOSpeedRatings"),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
        i((EXIF, "LightSource"), Kind::Simple(Prim::Integer));
        i((EXIF, "MaxApertureValue"), types::RATIONAL);
        i((EXIF, "MeteringMode"), Kind::Simple(Prim::Integer));
        i((EXIF, "OECF"), types::OECF_SFR);
        i((EXIF, "PixelXDimension"), Kind::Simple(Prim::Integer));
        i((EXIF, "PixelYDimension"), Kind::Simple(Prim::Integer));
        i((EXIF, "RelatedSoundFile"), Kind::Simple(Prim::Text));
        i((EXIF, "Saturation"), Kind::Simple(Prim::Integer));
        i((EXIF, "SceneCaptureType"), Kind::Simple(Prim::Integer));
        i((EXIF, "SceneType"), Kind::Simple(Prim::Integer));
        i((EXIF, "SensingMethod"), Kind::Simple(Prim::Integer));
        i((EXIF, "Sharpness"), Kind::Simple(Prim::Integer));
        i((EXIF, "ShutterSpeedValue"), types::RATIONAL);
        i((EXIF, "SpatialFrequencyResponse"), types::OECF_SFR);
        i((EXIF, "SpectralSensitivity"), Kind::Simple(Prim::Text));
        i(
            (EXIF, "SubjectArea"),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
        i((EXIF, "SubjectDistance"), types::RATIONAL);
        i((EXIF, "SubjectDistanceRange"), Kind::Simple(Prim::Integer));
        i(
            (EXIF, "SubjectLocation"),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
        i((EXIF, "UserComment"), types::LANGUAGE_ALTERNATIVE);
        i((EXIF, "WhiteBalance"), Kind::Simple(Prim::Integer));
        i((EXIF, "GPSAltitude"), types::RATIONAL);
        i((EXIF, "GPSAltitudeRef"), Kind::Simple(Prim::Integer));
        i((EXIF, "GPSAreaInformation"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSDestBearing"), types::RATIONAL);
        i((EXIF, "GPSDestBearingRef"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSDestDistance"), types::RATIONAL);
        i((EXIF, "GPSDestDistanceRef"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSDestLatitude"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSDestLongitude"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSDifferential"), Kind::Simple(Prim::Integer));
        i((EXIF, "GPSDOP"), types::RATIONAL);
        i((EXIF, "GPSImgDirection"), types::RATIONAL);
        i((EXIF, "GPSImgDirectionRef"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSLatitude"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSLongitude"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSMapDatum"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSMeasureMode"), Kind::Simple(Prim::Integer));
        i((EXIF, "GPSProcessingMethod"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSSatellites"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSSpeed"), types::RATIONAL);
        i((EXIF, "GPSSpeedRef"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSStatus"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSTimeStamp"), Kind::Simple(Prim::Date));
        i((EXIF, "GPSTrack"), types::RATIONAL);
        i((EXIF, "GPSTrackRef"), Kind::Simple(Prim::Text));
        i((EXIF, "GPSVersionID"), Kind::Simple(Prim::Text));
    }

    /*
     * TIFF namespace
     *
     * EXIF properties for TIFF-derived data."
     */
    {
        const TIFF: &str = "http://ns.adobe.com/tiff/1.0/";

        i((TIFF, "Artist"), types::PROPER_NAME);
        i(
            (TIFF, "BitsPerSample"),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
        i((TIFF, "Compression"), Kind::Simple(Prim::Integer));
        i((TIFF, "Copyright"), types::LANGUAGE_ALTERNATIVE);
        i((TIFF, "DateTime"), Kind::Simple(Prim::Date));
        i((TIFF, "ImageDescription"), types::LANGUAGE_ALTERNATIVE);
        i((TIFF, "ImageLength"), Kind::Simple(Prim::Integer));
        i((TIFF, "ImageWidth"), Kind::Simple(Prim::Integer));
        i((TIFF, "Make"), types::PROPER_NAME);
        i((TIFF, "Model"), types::PROPER_NAME);
        i((TIFF, "Orientation"), Kind::Simple(Prim::Integer));
        i(
            (TIFF, "PhotometricInterpretation"),
            Kind::Simple(Prim::Integer),
        );
        i((TIFF, "PlanarConfiguration"), Kind::Simple(Prim::Integer));
        i(
            (TIFF, "PrimaryChromaticities"),
            Kind::OrderedArray(&types::RATIONAL),
        );
        i(
            (TIFF, "ReferenceBlackWhite"),
            Kind::OrderedArray(&types::RATIONAL),
        );
        i((TIFF, "ResolutionUnit"), Kind::Simple(Prim::Integer));
        i((TIFF, "SamplesPerPixel"), Kind::Simple(Prim::Integer));
        i((TIFF, "Software"), types::AGENT_NAME);
        i(
            (TIFF, "TransferFunction"),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
        i((TIFF, "WhitePoint"), Kind::OrderedArray(&types::RATIONAL));
        i((TIFF, "XResolution"), types::RATIONAL);
        i((TIFF, "YResolution"), types::RATIONAL);
        i(
            (TIFF, "YCbCrCoefficients"),
            Kind::OrderedArray(&types::RATIONAL),
        );
        i((TIFF, "YCbCrPositioning"), Kind::Simple(Prim::Integer));
        i(
            (TIFF, "YCbCrSubSampling"),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
    }

    /*
     * Dublin Core namespace
     *
     * "The Dublin Core namespace provides a set of commonly used properties.
     * The names and usage shall be as defined in the Dublin Core Metadata
     * Element Set, created by the Dublin Core Metadata Initiative (DCMI)."
     */
    {
        const DC: &str = "http://purl.org/dc/elements/1.1/";

        i(
            (DC, "contributor"),
            Kind::UnorderedArray(&types::PROPER_NAME),
        );
        i((DC, "coverage"), Kind::Simple(Prim::Text));
        i((DC, "creator"), Kind::OrderedArray(&types::PROPER_NAME));
        i((DC, "date"), Kind::OrderedArray(&Kind::Simple(Prim::Date)));
        i((DC, "description"), types::LANGUAGE_ALTERNATIVE);
        i((DC, "format"), types::MIME_TYPE);
        i((DC, "identifier"), Kind::Simple(Prim::Text));
        i((DC, "language"), Kind::UnorderedArray(&types::LOCALE));
        i((DC, "publisher"), Kind::UnorderedArray(&types::PROPER_NAME));
        i(
            (DC, "relation"),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
        i((DC, "rights"), types::LANGUAGE_ALTERNATIVE);
        i((DC, "source"), Kind::Simple(Prim::Text));
        i(
            (DC, "subject"),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
        i((DC, "title"), types::LANGUAGE_ALTERNATIVE);
        i(
            (DC, "type"),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
    }

    /*
     * IPTC Core namespace
     *
     * "IPTC Photo Metadata provides data about photographs and the values can
     * be processed by software. Each individual metadata entity is called a
     * property and they are grouped into Administrative, Descriptive, and
     * Rights Related properties."
     */
    {
        // note: this is technically already implemented in IPTC4XMP, but we
        // also want to yield it when parsing XMP.
        const IPTC4XMP_CORE: &str = "http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/";

        i((IPTC4XMP_CORE, "CreatorContactInfo"), types::CONTACT_INFO);
        i(
            (IPTC4XMP_CORE, "IntellectualGenre"),
            Kind::Simple(Prim::Text),
        );
        i(
            (IPTC4XMP_CORE, "Scene"),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
        i((IPTC4XMP_CORE, "Location"), Kind::Simple(Prim::Text));
        i((IPTC4XMP_CORE, "CountryCode"), types::LOCALE);
        i(
            (IPTC4XMP_CORE, "SubjectCode"),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
    }

    // ...
}

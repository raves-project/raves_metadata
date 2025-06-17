use std::sync::LazyLock;

use rustc_hash::FxHashMap;

use crate::xmp_parsing_types::{XmpKind as Kind, XmpPrimitiveKind as Prim};

pub mod types;

/// A pair of a property's namespace URL and its element name.
///
/// Ex: ("http://ns.adobe.com/xap/1.0/", "CreateDate") for `xmp:CreateDate`
#[derive(Hash, PartialEq, Eq)]
pub struct XmpNamespaceNamePair(pub (&'static str, &'static str));

// this impl allows us to search the hashmap with borrowed expressions for any
// lifetime 'a.
//
// the compiler automatically coerces the 'static lifetime in the struct into
// 'a because of this method, as `HashMap<K, V, _>::get` uses a borrowed
// expression `&'any K` as its parameter. that's because &'static T is
// covariant over &'a T, for any 'a.
//
// in other words, we coerce 'static into 'a with this explicit trait impl
impl<'a> core::borrow::Borrow<(&'a str, &'a str)> for XmpNamespaceNamePair {
    fn borrow(&self) -> &(&'a str, &'a str) {
        &self.0
    }
}

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
    use XmpNamespaceNamePair as P;

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
        i(P((BASIC, "CreateDate")), Kind::Simple(Prim::Date));
        i(P((BASIC, "CreatorTool")), types::AGENT_NAME);
        i(
            P((BASIC, "Identifier")),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
        i(P((BASIC, "Label")), Kind::Simple(Prim::Text));
        i(P((BASIC, "MetadataDate")), Kind::Simple(Prim::Date));
        i(P((BASIC, "ModifyDate")), Kind::Simple(Prim::Date));
        i(P((BASIC, "Rating")), Kind::Simple(Prim::Real));
        i(P((BASIC, "BaseURL")), types::URL);
        i(P((BASIC, "Nickname")), Kind::Simple(Prim::Text));
        i(P((BASIC, "Thumbnails")), types::THUMBNAIL);
    }

    /*
     * XMP Media Management namespace
     *
     * "This namespace is primarily for use by digital asset management (DAM)
     * systems."
     */
    {
        const XMP_MM: &str = "http://ns.adobe.com/xap/1.0/mm/";
        i(P((XMP_MM, "DerivedFrom")), types::RESOURCE_REF);
        i(P((XMP_MM, "DocumentID")), types::GUID);
        i(P((XMP_MM, "InstanceID")), types::GUID);
        i(P((XMP_MM, "OriginalDocumentID")), types::GUID);
        i(P((XMP_MM, "RenditionClass")), types::RENDITION_CLASS);
        i(P((XMP_MM, "RenditionParams")), Kind::Simple(Prim::Text));
        i(
            P((XMP_MM, "History")),
            Kind::OrderedArray(&types::RESOURCE_EVENT),
        );
        i(
            P((XMP_MM, "Ingredients")),
            Kind::UnorderedArray(&types::RESOURCE_REF),
        );
        i(
            P((XMP_MM, "Pantry")),
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
            P((XMP_MM, "ManagedFrom")),
            Kind::UnorderedArray(&types::RESOURCE_REF),
        );
        i(P((XMP_MM, "Manager")), types::AGENT_NAME);
        i(P((XMP_MM, "ManageTo")), types::URI);
        i(P((XMP_MM, "ManageUI")), types::URI);
        i(P((XMP_MM, "ManagerVariant")), Kind::Simple(Prim::Text));
        i(P((XMP_MM, "VersionID")), Kind::Simple(Prim::Text));
        i(P((XMP_MM, "Versions")), Kind::OrderedArray(&types::VERSION));
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
        i(P((XMP_BJ, "JobRef")), Kind::UnorderedArray(&types::JOB));
    }

    /*
     * XMP Paged-Text namespace
     *
     * "The Paged-Text namespace is used for text appearing on a page in a
     * document."
     */
    {
        const XMP_TPG: &str = "http://ns.adobe.com/xap/1.0/t/pg/";
        i(
            P((XMP_TPG, "Colorants")),
            Kind::OrderedArray(&types::COLORANT),
        );
        i(P((XMP_TPG, "Fonts")), Kind::UnorderedArray(&types::FONT));
        i(P((XMP_TPG, "MaxPageSize")), types::DIMENSIONS);
        i(P((XMP_TPG, "NPages")), Kind::Simple(Prim::Integer));
        i(
            P((XMP_TPG, "PlateNames")),
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
        i(P((XMP_DM, "absPeakAudioFilePath")), types::URI);
        i(P((XMP_DM, "album")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "altTapeName")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "altTimecode")), types::TIMECODE);
        i(P((XMP_DM, "artist")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "audioChannelType")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "audioCompressor")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "audioSampleRate")), Kind::Simple(Prim::Integer));
        i(P((XMP_DM, "audioSampleType")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "beatSpliceParams")), types::BEAT_SPLICE_STRETCH);
        i(P((XMP_DM, "cameraAngle")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "cameraLabel")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "cameraModel")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "cameraMove")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "client")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "comment")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "composer")), Kind::Simple(Prim::Text));
        i(
            P((XMP_DM, "contributedMedia")),
            Kind::UnorderedArray(&types::MEDIA),
        );
        i(P((XMP_DM, "director")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "directorPhotography")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "duration")), types::TIME);
        i(P((XMP_DM, "engineer")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "fileDataRate")), types::RATIONAL);
        i(P((XMP_DM, "genre")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "good")), Kind::Simple(Prim::Boolean));
        i(P((XMP_DM, "instrument")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "introTime")), types::TIME);
        i(P((XMP_DM, "key")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "logComment")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "loop")), Kind::Simple(Prim::Boolean));
        i(P((XMP_DM, "numberOfBeats")), Kind::Simple(Prim::Real));
        i(P((XMP_DM, "markers")), Kind::OrderedArray(&types::MARKER));
        i(P((XMP_DM, "outCue")), types::TIME);
        i(P((XMP_DM, "projectName")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "projectRef")), types::PROJECT_LINK);
        i(P((XMP_DM, "pullDown")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "relativePeakAudioFilePath")), types::URI);
        i(P((XMP_DM, "relativeTimestamp")), types::TIME);
        i(P((XMP_DM, "releaseDate")), Kind::Simple(Prim::Date));
        i(P((XMP_DM, "resampleParams")), types::RESAMPLE_STRETCH);
        i(P((XMP_DM, "scaleType")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "scene")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "shotDate")), Kind::Simple(Prim::Date));
        i(P((XMP_DM, "shotDay")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "shotLocation")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "shotName")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "shotNumber")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "shotSize")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "speakerPlacement")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "startTimecode")), types::TIMECODE);
        i(P((XMP_DM, "stretchMode")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "takeNumber")), Kind::Simple(Prim::Integer));
        i(P((XMP_DM, "tapeName")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "tempo")), Kind::Simple(Prim::Real));
        i(P((XMP_DM, "timeScaleParams")), types::TIME_SCALE_STRETCH);
        i(P((XMP_DM, "timeSignature")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "trackNumber")), Kind::Simple(Prim::Integer));
        i(P((XMP_DM, "Tracks")), Kind::UnorderedArray(&types::TRACK));
        i(P((XMP_DM, "videoAlphaMode")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "videoAlphaPremultipleColor")), types::COLORANT);
        i(
            P((XMP_DM, "videoAlphaUnityIsTransparent")),
            Kind::Simple(Prim::Boolean),
        );
        i(P((XMP_DM, "videoColorSpace")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "videoCompressor")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "videoFieldOrder")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "videoFrameRate")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "videoFrameSize")), types::DIMENSIONS);
        i(P((XMP_DM, "videoPixelAspectRatio")), types::RATIONAL);
        i(P((XMP_DM, "videoPixelDepth")), Kind::Simple(Prim::Text));
        i(
            P((XMP_DM, "partOfCompilation")),
            Kind::Simple(Prim::Boolean),
        );
        i(P((XMP_DM, "lyrics")), Kind::Simple(Prim::Text));
        i(P((XMP_DM, "discNumber")), Kind::Simple(Prim::Text));
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
        i(P((XMP_RIGHTS, "Certificate")), Kind::Simple(Prim::Text));
        i(P((XMP_RIGHTS, "Marked")), Kind::Simple(Prim::Boolean));
        i(
            P((XMP_RIGHTS, "Owner")),
            Kind::UnorderedArray(&types::PROPER_NAME),
        );
        i(P((XMP_RIGHTS, "UsageTerms")), types::LANGUAGE_ALTERNATIVE);
        i(P((XMP_RIGHTS, "WebStatement")), Kind::Simple(Prim::Text));
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
        i(P((PDF, "Keywords")), Kind::Simple(Prim::Text));
        i(P((PDF, "PDFVersion")), Kind::Simple(Prim::Text));
        i(P((PDF, "Producer")), types::AGENT_NAME);

        // "True when the document has been trapped." ???
        //
        // that sounds so scary man D;
        i(P((PDF, "Trapped")), Kind::Simple(Prim::Boolean));
    }

    /*
     * Photoshop namespace
     *
     * "This namespace specifies properties used by Adobe Photoshop."
     */
    {
        pub const PHOTOSHOP: &str = "http://ns.adobe.com/photoshop/1.0/";

        i(P((PHOTOSHOP, "ColorMode")), Kind::Simple(Prim::Integer));
        i(
            P((PHOTOSHOP, "DocumentAncestors")),
            Kind::UnorderedArray(&types::ANCESTOR),
        );
        i(P((PHOTOSHOP, "History")), Kind::Simple(Prim::Text));
        i(P((PHOTOSHOP, "ICCProfile")), Kind::Simple(Prim::Text));
        i(
            P((PHOTOSHOP, "TextLayers")),
            Kind::OrderedArray(&types::LAYER),
        );
        i(P((PHOTOSHOP, "AuthorsPosition")), Kind::Simple(Prim::Text));
        i(P((PHOTOSHOP, "CaptionWriter")), types::PROPER_NAME);
        i(P((PHOTOSHOP, "Category")), Kind::Simple(Prim::Text));
        i(P((PHOTOSHOP, "City")), Kind::Simple(Prim::Text));
        i(P((PHOTOSHOP, "Country")), Kind::Simple(Prim::Text));
        i(P((PHOTOSHOP, "Credit")), Kind::Simple(Prim::Text));
        i(P((PHOTOSHOP, "DateCreated")), Kind::Simple(Prim::Date));
        i(P((PHOTOSHOP, "Headline")), Kind::Simple(Prim::Text));
        i(P((PHOTOSHOP, "Instructions")), Kind::Simple(Prim::Text));
        i(P((PHOTOSHOP, "Source")), Kind::Simple(Prim::Text));
        i(P((PHOTOSHOP, "State")), Kind::Simple(Prim::Text));
        i(
            P((PHOTOSHOP, "SupplementalCategories")),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
        i(
            P((PHOTOSHOP, "TransmissionReference")),
            Kind::Simple(Prim::Text),
        );
        i(P((PHOTOSHOP, "Urgency")), Kind::Simple(Prim::Integer));
    }

    /*
     * Camera Raw namespace
     *
     * "This namespace specifies settings associated with image files produced
     * in camera raw mode."
     */
    {
        pub const CRS: &str = "http://ns.adobe.com/camera-raw-settings/1.0/";

        i(P((CRS, "AutoBrightness")), Kind::Simple(Prim::Boolean));
        i(P((CRS, "AutoContrast")), Kind::Simple(Prim::Boolean));
        i(P((CRS, "AutoExposure")), Kind::Simple(Prim::Boolean));
        i(P((CRS, "AutoShadows")), Kind::Simple(Prim::Boolean));
        i(P((CRS, "BlueHue")), Kind::Simple(Prim::Integer));
        i(P((CRS, "BlueSaturation")), Kind::Simple(Prim::Integer));
        i(P((CRS, "Brightness")), Kind::Simple(Prim::Integer));
        i(P((CRS, "CameraProfile")), Kind::Simple(Prim::Text));
        i(
            P((CRS, "ChromaticAberrationB")),
            Kind::Simple(Prim::Integer),
        );
        i(
            P((CRS, "ChromaticAberrationR")),
            Kind::Simple(Prim::Integer),
        );
        i(P((CRS, "ColorNoiseReduction")), Kind::Simple(Prim::Integer));
        i(P((CRS, "Contrast")), Kind::Simple(Prim::Integer));
        i(P((CRS, "CropTop")), Kind::Simple(Prim::Real));
        i(P((CRS, "CropLeft")), Kind::Simple(Prim::Real));
        i(P((CRS, "CropBottom")), Kind::Simple(Prim::Real));
        i(P((CRS, "CropRight")), Kind::Simple(Prim::Real));
        i(P((CRS, "CropAngle")), Kind::Simple(Prim::Real));
        i(P((CRS, "CropWidth")), Kind::Simple(Prim::Real));
        i(P((CRS, "CropHeight")), Kind::Simple(Prim::Real));
        i(P((CRS, "CropUnits")), Kind::Simple(Prim::Integer));
        i(P((CRS, "Exposure")), Kind::Simple(Prim::Real));
        i(P((CRS, "GreenHue")), Kind::Simple(Prim::Integer));
        i(P((CRS, "GreenSaturation")), Kind::Simple(Prim::Integer));
        i(P((CRS, "HasCrop")), Kind::Simple(Prim::Boolean));
        i(P((CRS, "HasSettings")), Kind::Simple(Prim::Boolean));
        i(P((CRS, "LuminanceSmoothing")), Kind::Simple(Prim::Integer));
        i(P((CRS, "RawFileName")), Kind::Simple(Prim::Text));
        i(P((CRS, "RedHue")), Kind::Simple(Prim::Integer));
        i(P((CRS, "RedSaturation")), Kind::Simple(Prim::Integer));
        i(P((CRS, "Saturation")), Kind::Simple(Prim::Integer));
        i(P((CRS, "Shadows")), Kind::Simple(Prim::Integer));
        i(P((CRS, "ShadowTint")), Kind::Simple(Prim::Integer));
        i(P((CRS, "Sharpness")), Kind::Simple(Prim::Integer));
        i(P((CRS, "Temperature")), Kind::Simple(Prim::Integer));
        i(P((CRS, "Tint")), Kind::Simple(Prim::Integer));
        i(
            P((CRS, "ToneCurve")),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
        i(P((CRS, "ToneCurveName")), Kind::Simple(Prim::Text));
        i(P((CRS, "Version")), Kind::Simple(Prim::Text));
        i(P((CRS, "VignetteAmount")), Kind::Simple(Prim::Integer));
        i(P((CRS, "VignetteMidpoint")), Kind::Simple(Prim::Integer));
        i(P((CRS, "WhiteBalance")), Kind::Simple(Prim::Text));
    }

    /*
     * EXIF namespace
     *
     * "EXIF Schema For EXIF-Specific Properties. These properties defined
     * solely by EXIF."
     */
    {
        pub const EXIF: &str = "http://ns.adobe.com/exif/1.0/";

        i(P((EXIF, "ApertureValue")), types::RATIONAL);
        i(P((EXIF, "BrightnessValue")), types::RATIONAL);
        i(P((EXIF, "CFAPattern")), types::CFA_PATTERN);
        i(P((EXIF, "ColorSpace")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "CompressedBitsPerPixel")), types::RATIONAL);
        i(P((EXIF, "Contrast")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "CustomRendered")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "DateTimeDigitized")), Kind::Simple(Prim::Date));
        i(P((EXIF, "DateTimeOriginal")), Kind::Simple(Prim::Date));
        i(
            P((EXIF, "DeviceSettingDescription")),
            types::DEVICE_SETTINGS,
        );
        i(P((EXIF, "DigitalZoomRatio")), types::RATIONAL);
        i(P((EXIF, "ExifVersion")), Kind::Simple(Prim::Text));
        i(P((EXIF, "ExposureBiasValue")), types::RATIONAL);
        i(P((EXIF, "ExposureIndex")), types::RATIONAL);
        i(P((EXIF, "ExposureMode")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "ExposureProgram")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "ExposureTime")), types::RATIONAL);
        i(P((EXIF, "FileSource")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "Flash")), types::FLASH);
        i(P((EXIF, "FlashEnergy")), types::RATIONAL);
        i(P((EXIF, "FlashpixVersion")), Kind::Simple(Prim::Text));
        i(P((EXIF, "FNumber")), types::RATIONAL);
        i(P((EXIF, "FocalLength")), types::RATIONAL);
        i(
            P((EXIF, "FocalLengthIn35mmFilm")),
            Kind::Simple(Prim::Integer),
        );
        i(
            P((EXIF, "FocalPlaneResolutionUnit")),
            Kind::Simple(Prim::Integer),
        );
        i(P((EXIF, "FocalPlaneXResolution")), types::RATIONAL);
        i(P((EXIF, "FocalPlaneYResolution")), types::RATIONAL);
        i(P((EXIF, "GainControl")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "ImageUniqueID")), Kind::Simple(Prim::Text));
        i(
            P((EXIF, "ISOSpeedRatings")),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
        i(P((EXIF, "LightSource")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "MaxApertureValue")), types::RATIONAL);
        i(P((EXIF, "MeteringMode")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "OECF")), types::OECF_SFR);
        i(P((EXIF, "PixelXDimension")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "PixelYDimension")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "RelatedSoundFile")), Kind::Simple(Prim::Text));
        i(P((EXIF, "Saturation")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "SceneCaptureType")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "SceneType")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "SensingMethod")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "Sharpness")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "ShutterSpeedValue")), types::RATIONAL);
        i(P((EXIF, "SpatialFrequencyResponse")), types::OECF_SFR);
        i(P((EXIF, "SpectralSensitivity")), Kind::Simple(Prim::Text));
        i(
            P((EXIF, "SubjectArea")),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
        i(P((EXIF, "SubjectDistance")), types::RATIONAL);
        i(
            P((EXIF, "SubjectDistanceRange")),
            Kind::Simple(Prim::Integer),
        );
        i(
            P((EXIF, "SubjectLocation")),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
        i(P((EXIF, "UserComment")), types::LANGUAGE_ALTERNATIVE);
        i(P((EXIF, "WhiteBalance")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "GPSAltitude")), types::RATIONAL);
        i(P((EXIF, "GPSAltitudeRef")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "GPSAreaInformation")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSDestBearing")), types::RATIONAL);
        i(P((EXIF, "GPSDestBearingRef")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSDestDistance")), types::RATIONAL);
        i(P((EXIF, "GPSDestDistanceRef")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSDestLatitude")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSDestLongitude")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSDifferential")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "GPSDOP")), types::RATIONAL);
        i(P((EXIF, "GPSImgDirection")), types::RATIONAL);
        i(P((EXIF, "GPSImgDirectionRef")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSLatitude")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSLongitude")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSMapDatum")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSMeasureMode")), Kind::Simple(Prim::Integer));
        i(P((EXIF, "GPSProcessingMethod")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSSatellites")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSSpeed")), types::RATIONAL);
        i(P((EXIF, "GPSSpeedRef")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSStatus")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSTimeStamp")), Kind::Simple(Prim::Date));
        i(P((EXIF, "GPSTrack")), types::RATIONAL);
        i(P((EXIF, "GPSTrackRef")), Kind::Simple(Prim::Text));
        i(P((EXIF, "GPSVersionID")), Kind::Simple(Prim::Text));
    }

    /*
     * TIFF namespace
     *
     * EXIF properties for TIFF-derived data."
     */
    {
        const TIFF: &str = "http://ns.adobe.com/tiff/1.0/";

        i(P((TIFF, "Artist")), types::PROPER_NAME);
        i(
            P((TIFF, "BitsPerSample")),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
        i(P((TIFF, "Compression")), Kind::Simple(Prim::Integer));
        i(P((TIFF, "Copyright")), types::LANGUAGE_ALTERNATIVE);
        i(P((TIFF, "DateTime")), Kind::Simple(Prim::Date));
        i(P((TIFF, "ImageDescription")), types::LANGUAGE_ALTERNATIVE);
        i(P((TIFF, "ImageLength")), Kind::Simple(Prim::Integer));
        i(P((TIFF, "ImageWidth")), Kind::Simple(Prim::Integer));
        i(P((TIFF, "Make")), types::PROPER_NAME);
        i(P((TIFF, "Model")), types::PROPER_NAME);
        i(P((TIFF, "Orientation")), Kind::Simple(Prim::Integer));
        i(
            P((TIFF, "PhotometricInterpretation")),
            Kind::Simple(Prim::Integer),
        );
        i(
            P((TIFF, "PlanarConfiguration")),
            Kind::Simple(Prim::Integer),
        );
        i(
            P((TIFF, "PrimaryChromaticities")),
            Kind::OrderedArray(&types::RATIONAL),
        );
        i(
            P((TIFF, "ReferenceBlackWhite")),
            Kind::OrderedArray(&types::RATIONAL),
        );
        i(P((TIFF, "ResolutionUnit")), Kind::Simple(Prim::Integer));
        i(P((TIFF, "SamplesPerPixel")), Kind::Simple(Prim::Integer));
        i(P((TIFF, "Software")), types::AGENT_NAME);
        i(
            P((TIFF, "TransferFunction")),
            Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        );
        i(
            P((TIFF, "WhitePoint")),
            Kind::OrderedArray(&types::RATIONAL),
        );
        i(P((TIFF, "XResolution")), types::RATIONAL);
        i(P((TIFF, "YResolution")), types::RATIONAL);
        i(
            P((TIFF, "YCbCrCoefficients")),
            Kind::OrderedArray(&types::RATIONAL),
        );
        i(P((TIFF, "YCbCrPositioning")), Kind::Simple(Prim::Integer));
        i(
            P((TIFF, "YCbCrSubSampling")),
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
            P((DC, "contributor")),
            Kind::UnorderedArray(&types::PROPER_NAME),
        );
        i(P((DC, "coverage")), Kind::Simple(Prim::Text));
        i(P((DC, "creator")), Kind::OrderedArray(&types::PROPER_NAME));
        i(
            P((DC, "date")),
            Kind::OrderedArray(&Kind::Simple(Prim::Date)),
        );
        i(P((DC, "description")), types::LANGUAGE_ALTERNATIVE);
        i(P((DC, "format")), types::MIME_TYPE);
        i(P((DC, "identifier")), Kind::Simple(Prim::Text));
        i(P((DC, "language")), Kind::UnorderedArray(&types::LOCALE));
        i(
            P((DC, "publisher")),
            Kind::UnorderedArray(&types::PROPER_NAME),
        );
        i(
            P((DC, "relation")),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
        i(P((DC, "rights")), types::LANGUAGE_ALTERNATIVE);
        i(P((DC, "source")), Kind::Simple(Prim::Text));
        i(
            P((DC, "subject")),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
        i(P((DC, "title")), types::LANGUAGE_ALTERNATIVE);
        i(
            P((DC, "type")),
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

        i(
            P((IPTC4XMP_CORE, "CreatorContactInfo")),
            types::CONTACT_INFO,
        );
        i(
            P((IPTC4XMP_CORE, "IntellectualGenre")),
            Kind::Simple(Prim::Text),
        );
        i(
            P((IPTC4XMP_CORE, "Scene")),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
        i(P((IPTC4XMP_CORE, "Location")), Kind::Simple(Prim::Text));
        i(P((IPTC4XMP_CORE, "CountryCode")), types::LOCALE);
        i(
            P((IPTC4XMP_CORE, "SubjectCode")),
            Kind::UnorderedArray(&Kind::Simple(Prim::Text)),
        );
    }

    // ...
}

//! We can make "types" as constants in this module.
//!
//! We won't be reasoning about types in our codebase (just parsing XML
//! according to the noted primitives), but having some constants down here
//! might save some space and time... ;D

use crate::xmp_parsing_types::{
    XmpKind as Kind, XmpKindStructField as Field, XmpKindStructFieldIdent as Ident,
    XmpPrimitiveKind as Prim,
};

pub const AGENT_NAME: Kind = Kind::Simple(Prim::Text);
pub const ANCESTOR: Kind = Kind::Struct(&[Field {
    ident: Ident::Namespaced {
        field_name: "AncestorID",
        namespace: "http://ns.adobe.com/photoshop/1.0/",
    },
    ty: &URI,
}]);
pub const BEAT_SPLICE_STRETCH: Kind = Kind::Struct(&[
    Field {
        ident: Ident::Namespaced {
            field_name: "riseInDecibel",
            namespace: "http://ns.adobe.com/xmp/1.0/DynamicMedia/",
        },
        ty: &Kind::Simple(Prim::Real),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "riseInTimeDuration",
            namespace: "http://ns.adobe.com/xmp/1.0/DynamicMedia/",
        },
        ty: &TIME,
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "useFileBeatsMarker",
            namespace: "http://ns.adobe.com/xmp/1.0/DynamicMedia/",
        },
        ty: &Kind::Simple(Prim::Boolean),
    },
]);
pub const CFA_PATTERN: Kind = Kind::Struct({
    const NAMESPACE: &str = "http://ns.adobe.com/exif/1.0/";

    &[
        Field {
            ident: Ident::Namespaced {
                field_name: "Columns",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Integer),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "Rows",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Integer),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "Values",
                namespace: NAMESPACE,
            },
            ty: &Kind::OrderedArray(&Kind::Simple(Prim::Integer)),
        },
    ]
});
#[doc(alias = "COLORANTS")]
pub const COLORANT: Kind = Kind::Union {
    always: &[
        Field {
            ident: Ident::Namespaced {
                field_name: "mode",
                namespace: "http://ns.adobe.com/xap/1.0/g/",
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "swatchName",
                namespace: "http://ns.adobe.com/xap/1.0/g/",
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "swatchName",
                namespace: "http://ns.adobe.com/xap/1.0/g/",
            },
            ty: &Kind::Simple(Prim::Text),
        },
    ],

    discriminant: Field {
        ident: Ident::Namespaced {
            field_name: "mode",
            namespace: "http://ns.adobe.com/xap/1.0/g/",
        },
        ty: &Kind::Simple(Prim::Text),
    },

    optional: &[
        // LAB
        (
            "LAB",
            &[
                Field {
                    ident: Ident::Namespaced {
                        field_name: "A",
                        namespace: "http://ns.adobe.com/xap/1.0/g/",
                    },
                    ty: &Kind::Simple(Prim::Integer),
                },
                Field {
                    ident: Ident::Namespaced {
                        field_name: "B",
                        namespace: "http://ns.adobe.com/xap/1.0/g/",
                    },
                    ty: &Kind::Simple(Prim::Integer),
                },
                Field {
                    ident: Ident::Namespaced {
                        field_name: "L",
                        namespace: "http://ns.adobe.com/xap/1.0/g/",
                    },
                    ty: &Kind::Simple(Prim::Real),
                },
            ],
        ),
        //
        // CMYK
        (
            "CMWK",
            &[
                Field {
                    ident: Ident::Namespaced {
                        field_name: "black",
                        namespace: "http://ns.adobe.com/xap/1.0/g/",
                    },
                    ty: &Kind::Simple(Prim::Real),
                },
                Field {
                    ident: Ident::Namespaced {
                        field_name: "cyan",
                        namespace: "http://ns.adobe.com/xap/1.0/g/",
                    },
                    ty: &Kind::Simple(Prim::Real),
                },
                Field {
                    ident: Ident::Namespaced {
                        field_name: "magenta",
                        namespace: "http://ns.adobe.com/xap/1.0/g/",
                    },
                    ty: &Kind::Simple(Prim::Real),
                },
                Field {
                    ident: Ident::Namespaced {
                        field_name: "yellow",
                        namespace: "http://ns.adobe.com/xap/1.0/g/",
                    },
                    ty: &Kind::Simple(Prim::Real),
                },
            ],
        ),
        //
        // RGB
        (
            "RGB",
            &[
                Field {
                    ident: Ident::Namespaced {
                        field_name: "blue",
                        namespace: "http://ns.adobe.com/xap/1.0/g/",
                    },
                    ty: &Kind::Simple(Prim::Integer),
                },
                Field {
                    ident: Ident::Namespaced {
                        field_name: "green",
                        namespace: "http://ns.adobe.com/xap/1.0/g/",
                    },
                    ty: &Kind::Simple(Prim::Integer),
                },
                Field {
                    ident: Ident::Namespaced {
                        field_name: "red",
                        namespace: "http://ns.adobe.com/xap/1.0/g/",
                    },
                    ty: &Kind::Simple(Prim::Integer),
                },
            ],
        ),
    ],
};
pub const CONTACT_INFO: Kind = Kind::Struct({
    const NAMESPACE: &str = "http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/";

    &[
        Field {
            ident: Ident::Namespaced {
                field_name: "CiAdrExtadr",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "CiAdrCity",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "CiAdrRegion",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "CiAdrPcode",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "CiAdrCtry",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "CiTelWork",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "CiEmailWork",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "CiUrlWork",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
    ]
});
pub const CUE_POINT_PARAM: Kind = Kind::Struct({
    const NAMESPACE: &str = "http://ns.adobe.com/xmp/1.0/DynamicMedia/";
    &[
        Field {
            ident: Ident::Namespaced {
                field_name: "key",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "value",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
    ]
});
pub const DEVICE_SETTINGS: Kind = Kind::Struct({
    const NAMESPACE: &str = "http://ns.adobe.com/exif/1.0/";

    &[
        Field {
            ident: Ident::Namespaced {
                field_name: "Columns",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Integer),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "Rows",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Integer),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "Values",
                namespace: NAMESPACE,
            },
            ty: &Kind::OrderedArray(&Kind::Simple(Prim::Text)),
        },
    ]
});
pub const DIMENSIONS: Kind = Kind::Struct(&[
    Field {
        ident: Ident::Namespaced {
            field_name: "h",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Dimensions#",
        },
        ty: &Kind::Simple(Prim::Real),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "w",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Dimensions#",
        },
        ty: &Kind::Simple(Prim::Real),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "unit",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Dimensions#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
]);
pub const FLASH: Kind = Kind::Struct({
    const NAMESPACE: &str = "http://ns.adobe.com/exif/1.0/";

    &[
        Field {
            ident: Ident::Namespaced {
                field_name: "Fired",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Boolean),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "Function",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Boolean),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "Mode",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Integer),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "RedEyeMode",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Boolean),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "Return",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Integer),
        },
    ]
});
pub const FONT: Kind = Kind::Struct(&[
    Field {
        ident: Ident::Namespaced {
            field_name: "childFontFiles",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Font#",
        },
        ty: &Kind::OrderedArray(&Kind::Simple(Prim::Text)),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "composite",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Font#",
        },
        ty: &Kind::Simple(Prim::Boolean),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "fontFace",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Font#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "fontFamily",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Font#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "fontFileName",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Font#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "fontName",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Font#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "fontType",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Font#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "versionString",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Font#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
]);
/// This is used by the standard every so often, but using its data will
/// require extra parsing that isn't easily expressible here.
///
/// We'll leave that to users for now.
pub const FRAME_COUNT: Kind = Kind::StructUnspecifiedFields {
    required_fields: &[],
};
/// Similar to [`FRAME_COUNT`] - leaving the parsing to users for now.
pub const FRAME_RATE: Kind = Kind::StructUnspecifiedFields {
    required_fields: &[],
};
pub const GUID: Kind = Kind::Simple(Prim::Text);
pub const JOB: Kind = Kind::Struct(&[
    Field {
        ident: Ident::Namespaced {
            field_name: "id",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Job#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "name",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Job#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "url",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Job#",
        },
        ty: &URL,
    },
]);
/// note: The type isn't explicitly stated to be "Text" on this element,
/// but we can safely assume so from real-world samples.
#[doc(alias = "LANGUAGE_ALTERNATIVES")]
pub const LANGUAGE_ALTERNATIVE: Kind = Kind::Alternatives(&Kind::Simple(Prim::Text));
pub const LAYER: Kind = Kind::Struct({
    const NAMESPACE: &str = "http://ns.adobe.com/photoshop/1.0/";
    &[
        Field {
            ident: Ident::Namespaced {
                field_name: "LayerName",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "LayerText",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
    ]
});
pub const LOCALE: Kind = Kind::Simple(Prim::Text);
pub const MEDIA: Kind = Kind::Struct({
    const NAMESPACE: &str = "http://ns.adobe.com/xmp/1.0/DynamicMedia/";
    &[
        Field {
            ident: Ident::Namespaced {
                field_name: "duration",
                namespace: NAMESPACE,
            },
            ty: &TIME,
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "managed",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Boolean),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "path",
                namespace: NAMESPACE,
            },
            ty: &URI,
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "startTime",
                namespace: NAMESPACE,
            },
            ty: &TIME,
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "track",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "webStatement",
                namespace: NAMESPACE,
            },
            ty: &URI,
        },
    ]
});
pub const MARKER: Kind = Kind::Struct({
    pub const NAMESPACE: &str = "http://ns.adobe.com/xmp/1.0/DynamicMedia/";
    &[
        Field {
            ident: Ident::Namespaced {
                field_name: "comment",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "cuePointParams",
                namespace: NAMESPACE,
            },
            ty: &Kind::OrderedArray(&CUE_POINT_PARAM),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "cuePointType",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "duration",
                namespace: NAMESPACE,
            },
            ty: &FRAME_COUNT,
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "location",
                namespace: NAMESPACE,
            },
            ty: &URI,
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "name",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "probability",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Real),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "speaker",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "startTime",
                namespace: NAMESPACE,
            },
            ty: &FRAME_COUNT,
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "target",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "type",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
    ]
});
pub const MIME_TYPE: Kind = Kind::Simple(Prim::Text);
/// Opto-electronic conversion function + spatial frequency response
/// information.
///
/// Seems to be used with certain kinds of cameras, as it's required to
/// parse Exif.
pub const OECF_SFR: Kind = Kind::Struct({
    const NAMESPACE: &str = "http://ns.adobe.com/exif/1.0/";

    &[
        Field {
            ident: Ident::Namespaced {
                field_name: "Columns",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Integer),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "Names", // column names probably woulda been better lol
                namespace: NAMESPACE,
            },
            ty: &Kind::OrderedArray(&Kind::Simple(Prim::Text)),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "Rows",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Integer),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "Values",
                namespace: NAMESPACE,
            },
            ty: &Kind::OrderedArray(&RATIONAL),
        },
    ]
});
pub const PROJECT_LINK: Kind = Kind::Struct({
    const NAMESPACE: &str = "http://ns.adobe.com/xmp/1.0/DynamicMedia/";
    &[
        Field {
            ident: Ident::Namespaced {
                field_name: "path",
                namespace: NAMESPACE,
            },
            ty: &URI,
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "type",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
    ]
});
pub const PROPER_NAME: Kind = Kind::Simple(Prim::Text);
pub const RATIONAL: Kind = Kind::Simple(Prim::Text);
pub const RESAMPLE_STRETCH: Kind = Kind::Struct(&[Field {
    ident: Ident::Namespaced {
        field_name: "quality",
        namespace: "http://ns.adobe.com/xmp/1.0/DynamicMedia/",
    },
    ty: &Kind::Simple(Prim::Text),
}]);
pub const RESOURCE_REF: Kind = Kind::Struct(&[
    Field {
        ident: Ident::Namespaced {
            field_name: "documentID",
            namespace: "http://ns.adobe.com/xap/1.0/sType/ResourceRef#",
        },
        ty: &GUID,
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "filePath",
            namespace: "http://ns.adobe.com/xap/1.0/sType/ResourceRef#",
        },
        ty: &URI,
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "instanceID",
            namespace: "http://ns.adobe.com/xap/1.0/sType/ResourceRef#",
        },
        ty: &GUID,
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "renditionClass",
            namespace: "http://ns.adobe.com/xap/1.0/sType/ResourceRef#",
        },
        ty: &RENDITION_CLASS,
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "renditionParam",
            namespace: "http://ns.adobe.com/xap/1.0/sType/ResourceRef#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
]);
pub const RENDITION_CLASS: Kind = Kind::Simple(Prim::Text);
pub const RESOURCE_EVENT: Kind = Kind::Struct(&[
    Field {
        ident: Ident::Namespaced {
            field_name: "action",
            namespace: "http://ns.adobe.com/xap/1.0/sType/ResourceEvent#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "changed",
            namespace: "http://ns.adobe.com/xap/1.0/sType/ResourceEvent#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "instanceID",
            namespace: "http://ns.adobe.com/xap/1.0/sType/ResourceEvent#",
        },
        ty: &GUID,
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "parameters",
            namespace: "http://ns.adobe.com/xap/1.0/sType/ResourceEvent#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "softwareAgent",
            namespace: "http://ns.adobe.com/xap/1.0/sType/ResourceEvent#",
        },
        ty: &AGENT_NAME,
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "when",
            namespace: "http://ns.adobe.com/xap/1.0/sType/ResourceEvent#",
        },
        ty: &Kind::Simple(Prim::Date),
    },
]);
pub const THUMBNAIL: Kind = Kind::Struct(&[
    Field {
        ident: Ident::Namespaced {
            field_name: "format",
            namespace: "http://ns.adobe.com/xap/1.0/g/img/",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "height",
            namespace: "http://ns.adobe.com/xap/1.0/g/img/",
        },
        ty: &Kind::Simple(Prim::Integer),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "width",
            namespace: "http://ns.adobe.com/xap/1.0/g/img/",
        },
        ty: &Kind::Simple(Prim::Integer),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "image",
            namespace: "http://ns.adobe.com/xap/1.0/g/img/",
        },
        ty: &Kind::Simple(Prim::Text),
    },
]);
pub const TIME: Kind = Kind::Struct(&[
    Field {
        ident: Ident::Namespaced {
            field_name: "scale",
            namespace: "http://ns.adobe.com/xmp/1.0/DynamicMedia/",
        },
        ty: &RATIONAL,
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "value",
            namespace: "http://ns.adobe.com/xmp/1.0/DynamicMedia/",
        },
        ty: &Kind::Simple(Prim::Integer),
    },
]);
pub const TIMECODE: Kind = Kind::Struct(&[
    Field {
        ident: Ident::Namespaced {
            field_name: "timeFormat",
            namespace: "http://ns.adobe.com/xmp/1.0/DynamicMedia/",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "timeValue",
            namespace: "http://ns.adobe.com/xmp/1.0/DynamicMedia/",
        },
        ty: &Kind::Simple(Prim::Text),
    },
]);
pub const TIME_SCALE_STRETCH: Kind = Kind::Struct({
    const NAMESPACE: &str = "http://ns.adobe.com/xmp/1.0/DynamicMedia/";
    &[
        Field {
            ident: Ident::Namespaced {
                field_name: "frameOverlappingPercentage",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Real),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "frameSize",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Real),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "quality",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
    ]
});
pub const TRACK: Kind = Kind::Struct({
    const NAMESPACE: &str = "http://ns.adobe.com/xmp/1.0/DynamicMedia/";
    &[
        Field {
            ident: Ident::Namespaced {
                field_name: "frameRate",
                namespace: NAMESPACE,
            },
            ty: &FRAME_RATE,
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "markers",
                namespace: NAMESPACE,
            },
            ty: &Kind::OrderedArray(&MARKER),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "trackName",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
        Field {
            ident: Ident::Namespaced {
                field_name: "trackType",
                namespace: NAMESPACE,
            },
            ty: &Kind::Simple(Prim::Text),
        },
    ]
});
pub const URI: Kind = Kind::Simple(Prim::Text);
pub const URL: Kind = Kind::Simple(Prim::Text);
pub const VERSION: Kind = Kind::Struct(&[
    Field {
        ident: Ident::Namespaced {
            field_name: "comments",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Version#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "event",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Version#",
        },
        ty: &RESOURCE_EVENT,
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "modifier",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Version#",
        },
        ty: &PROPER_NAME,
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "modifyDate",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Version#",
        },
        ty: &Kind::Simple(Prim::Date),
    },
    Field {
        ident: Ident::Namespaced {
            field_name: "version",
            namespace: "http://ns.adobe.com/xap/1.0/sType/Version#",
        },
        ty: &Kind::Simple(Prim::Text),
    },
]);

use std::borrow::Cow;

use raves_metadata_types::{
    xmp::XmpValue,
    xmp_parsing_types::{XmpKind as Kind, XmpKindStructField as Field, XmpPrimitiveKind as Prim},
};
use xmltree::Element;

use crate::xmp::{
    error::{XmpElementResult, XmpParsingError},
    value::{XmpElementExt, structs::value_struct_field},
};

/// Parses an element's value as a union.
pub fn value_union<'xml>(
    element: &'xml Element,
    always: &'static [Field],
    discriminant: &'static Field,
    optional: &'static [(&'static str, &'static [Field])],
) -> XmpElementResult<'xml> {
    // ok! some things to note:
    //
    // - `always` refers to the union's fields that are present on each
    //   variant.
    //      - when they're missing, we only warn - no erroring!
    // - `discriminant` is the field in `always` that we use to tell which
    //   which tag we're using for the union.
    //      - this is required - we'll error without it.
    //      - also, its type must always be `Text` for now. we may change
    //        this in the future if we find non-text discriminants
    // - `optional` is the collection of fields we expect to have for a
    //   a given `discriminant` (as text).

    // ensure the `discriminant` can be represented as text
    let Field {
        ident: _,
        ty: Kind::Simple(Prim::Text),
    } = discriminant
    else {
        log::error!(
            "Unable to parse union. Discriminant was a non-text type: \
                {discriminant:#?}"
        );
        return Err(XmpParsingError::UnionDiscriminantWasntText {
            element_name: Cow::from(&element.name),
            discriminant_kind: discriminant,
        });
    };

    // we're about to find all the fields.
    //
    // first, for perf, let's grab:
    //
    // - a list of the `always` field ns + names
    // - and the same list, but for `optional`
    let known_pairs: Vec<(&Field, _)> = {
        // first, the "always" pairs
        let always_pairs = always
            .iter()
            .chain([discriminant])
            .map(|f| (f, &f.ident))
            .map(|(f, f_ident)| (f, (f_ident.ns(), f_ident.name())));

        // then the "optional" pairs
        let optional_pairs = optional.iter().flat_map(|(_, slic)| {
            slic.iter()
                .map(|f| (f, &f.ident))
                .map(|(f, f_ident)| (f, (f_ident.ns(), f_ident.name())))
        });

        // chain em together + collect into a vec
        always_pairs.chain(optional_pairs).collect()
    };

    // find all fields
    let mut expected_fields: Vec<_> = Vec::with_capacity(always.len());
    let mut unexpected_fields: Vec<_> = Vec::with_capacity(element.children.len() - always.len());
    for c in element.children.iter().flat_map(|c| c.as_element()) {
        // grab namespace + name
        let (c_ns, c_name) = (&c.namespace, &c.name);

        // check if we know this field
        let mut known_field = false;
        for (field, (known_ns, known_name)) in &known_pairs {
            // if the namespaces don't match, it's not this pair!
            //
            // sorry for this being so sloppy - need to compare both the
            // Some + None cases, and we don't own the &'static str...
            let ns_mismatch = match c_ns {
                Some(c_ns) => Some(c_ns.as_str()) != known_ns.map(|k: &str| k),
                None => known_ns.is_some(),
            };
            if ns_mismatch {
                log::trace!(
                    "Namespaces don't match - not a known field. found: `{c_ns:?}`, want: `{known_ns:?}`."
                );
                continue;
            }

            // if the names don't match, we can skip the pair
            if c_name != *known_name {
                log::trace!(
                    "Names don't match - not a known field. \
                    found: {c_name} \
                    want: {known_name}"
                );
                continue;
            }
            known_field = true; // still here - we're a known field.

            // if parsing works, push the result into the vec
            if let Some(parsed_field) = value_struct_field(c, Some(field)) {
                expected_fields.push(parsed_field);
            }
        }

        // this was an unexpected field...
        //
        // parse it and push it accordingly
        if !known_field && let Some(parsed_c) = value_struct_field(c, None) {
            unexpected_fields.push(parsed_c);
        }
    }

    // find the discriminant (or err)
    let Some(found_discriminant) = expected_fields.iter().find(|f| {
        // the names have to match
        let names_match = f.ident() == discriminant.ident.name();
        log::trace!("finding discrim... `names_match: bool = {names_match}`");
        if !names_match {
            log::trace!(
                "name for this field was: `{}`. expected: `{}`",
                f.ident(),
                discriminant.ident.name()
            );
        }

        // so does the namespace
        let namespaces_match = match f.namespace() {
            Some(ref s) => Some(&**s) == discriminant.ident.ns(),
            None => discriminant.ident.ns().is_none(),
        };
        log::trace!("finding discrim... `namespaces_match: bool = {namespaces_match}`");
        if !namespaces_match {
            log::trace!(
                "namespace for this field was: `{:?}`. expected: `{:?}`",
                f.namespace(),
                discriminant.ident.ns(),
            );
        }

        names_match && namespaces_match
    }) else {
        log::error!(
            "Parsed union, but couldn't find discriminant!
                    - discriminant: {discriminant:#?}
                    - expected_fields: {expected_fields:#?}
                    - unexpected_fields: {unexpected_fields:#?}"
        );
        return Err(XmpParsingError::UnionNoDiscriminant {
            element_name: Cow::from(&element.name),
        });
    };

    // return a parsed-out union
    element.to_xmp_element(XmpValue::Union {
        discriminant: Box::new(found_discriminant.clone()),
        expected_fields,
        unexpected_fields,
    })
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::{
        xmp::{XmpElement, XmpPrimitive, XmpValue, XmpValueStructField},
        xmp_parsing_types::XmpKind,
    };
    use xmltree::Element;

    use crate::xmp::value::unions::value_union;

    #[test]
    fn union_colorant() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        //

        let xml = r#"<rdf:li rdf:parseType="Resource" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:xmpG="http://ns.adobe.com/xap/1.0/g/">
    <xmpG:swatchName>black</xmpG:swatchName>
    <xmpG:mode>CMYK</xmpG:mode>
    <xmpG:type>PROCESS</xmpG:type>

    <xmpG:cyan>100</xmpG:cyan>
    <xmpG:magenta>100</xmpG:magenta>
    <xmpG:yellow>100</xmpG:yellow>
    <xmpG:black>100</xmpG:black>
</rdf:li>"#;

        let element = Element::parse(xml.as_bytes()).expect("xmltree should parse");

        // grab the `Colorant` type from our types lib
        let colorant = &raves_metadata_types::xmp_parse_table::types::COLORANT;

        let XmpKind::Union {
            always,
            discriminant,
            optional,
        } = colorant
        else {
            panic!("`Colorant` is no longer a union I guess..?");
        };

        let parsed_union =
            value_union(&element, always, discriminant, optional).expect("should parse out union");

        assert_eq!(
            parsed_union,
            XmpElement {
                namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                prefix: "rdf".into(),
                name: "li".into(),
                value: XmpValue::Union {
                    discriminant: Box::new(XmpValueStructField::Value {
                        ident: "mode".into(),
                        namespace: Some("http://ns.adobe.com/xap/1.0/g/".into()),
                        value: XmpValue::Simple(XmpPrimitive::Text("CMYK".into()))
                    }),
                    expected_fields: vec![
                        // always fields: `swatchName` + `mode`
                        XmpValueStructField::Value {
                            ident: "swatchName".into(),
                            namespace: Some("http://ns.adobe.com/xap/1.0/g/".into()),
                            value: XmpValue::Simple(XmpPrimitive::Text("black".into()))
                        },
                        XmpValueStructField::Value {
                            ident: "mode".into(),
                            namespace: Some("http://ns.adobe.com/xap/1.0/g/".into()),
                            value: XmpValue::Simple(XmpPrimitive::Text("CMYK".into()))
                        },
                        XmpValueStructField::Value {
                            ident: "type".into(),
                            namespace: Some("http://ns.adobe.com/xap/1.0/g/".into()),
                            value: XmpValue::Simple(XmpPrimitive::Text("PROCESS".into()))
                        },
                        //
                        // optional fields (for mode::CMYK)
                        XmpValueStructField::Value {
                            ident: "cyan".into(),
                            namespace: Some("http://ns.adobe.com/xap/1.0/g/".into()),
                            value: XmpValue::Simple(XmpPrimitive::Real(100.0))
                        },
                        XmpValueStructField::Value {
                            ident: "magenta".into(),
                            namespace: Some("http://ns.adobe.com/xap/1.0/g/".into()),
                            value: XmpValue::Simple(XmpPrimitive::Real(100.0))
                        },
                        XmpValueStructField::Value {
                            ident: "yellow".into(),
                            namespace: Some("http://ns.adobe.com/xap/1.0/g/".into()),
                            value: XmpValue::Simple(XmpPrimitive::Real(100.0))
                        },
                        XmpValueStructField::Value {
                            ident: "black".into(),
                            namespace: Some("http://ns.adobe.com/xap/1.0/g/".into()),
                            value: XmpValue::Simple(XmpPrimitive::Real(100.0))
                        },
                    ],
                    unexpected_fields: vec![]
                }
            }
        );
    }
}

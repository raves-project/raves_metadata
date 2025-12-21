use raves_metadata_types::xmp::{
    XmpValue,
    parse_types::{XmpKind as Kind, XmpPrimitiveKind as Prim},
};
use xmltree::{Element, XMLNode};

use crate::xmp::{
    RDF_NAMESPACE,
    error::{XmpElementResult, XmpParsingError},
    value::{XmpElementExt, prims::parse_primitive},
};

/// Parses an element's value as a list of alternatives.
///
/// These are generally represented by `rdf:Alt`, with each inner `rdf:li`
/// storing one possible display value.
///
/// Alternative collections usually look like the following:
///
/// ```xml
/// <ns:element>
///     <rdf:Alt>
///         <rdf:li xml:lang="x-default">XMP - Extensible Metadata Platform</rdf:li>
///         <rdf:li xml:lang="en-us">XMP - Extensible Metadata Platform</rdf:li>
///         <rdf:li xml:lang="fr-fr">XMP - Une Platforme Extensible pour les Métadonnées</rdf:li>
///         <rdf:li xml:lang="it-it">XMP - Piattaforma Estendibile di Metadata</rdf:li>
///     </rdf:Alt>
/// </ns:element>
/// ```
///
/// We should pick a matching `xml:lang` to what a user asks for, or,
/// otherwise, grab the `x-default` option.
pub fn value_alternatives(
    element: &Element,
    _maybe_ty: Option<&'static Kind>, // TODO: use for better parsing
) -> XmpElementResult {
    // try to find an `rdf:Alt`
    let alt: &Element = element
        .children
        .iter()
        .flat_map(|cn: &XMLNode| cn.as_element())
        .flat_map(|c: &Element| Some((c, c.namespace.clone()?)))
        .find(|(e, ns)| ns.as_str() == RDF_NAMESPACE && e.name.as_str() == "Alt")
        .map(|(e, _)| e)
        .ok_or(XmpParsingError::ArrayNoInnerCollectionType {
            element_name: element.name.clone(),
            children: element.children.clone(),
        })?;

    // each `rdf:li` sub-element will become a tuple: (parsed_inner, case),
    // where:
    //
    // - `parsed_inner` represents the actual parsed data, and
    // - `case` says which case we are (e.g. `x-default`)
    // grab a list of `rdf:li`
    let lis = alt
        .children
        .iter()
        .flat_map(|cn: &XMLNode| cn.as_element())
        .filter(|maybe_li| {
            // ensure we've got a namespace
            let Some(ref ns) = maybe_li.namespace else {
                log::warn!(
                    "sub-element of `rdf:Alt` was expected to be `rdf:li`, \
                        but had no namespace. element name: `{}`",
                    &maybe_li.name
                );
                return false;
            };

            // actually check namespace
            ns.as_str() == RDF_NAMESPACE && maybe_li.name.as_str() == "li"
        });

    // parse each `rdf:li` into a (tag, `XmpValue`) pair
    let parsed_lis: Vec<_> = lis
        .flat_map(|li| {
            // we need to know which case we're workin with.
            //
            // each li should have a case tag
            let maybe_tag_primitive = li.attributes.iter().find(|(keys, _)| {
                // check prefix (TODO: does `xml` have a namespace URI?)
                if keys
                    .prefix
                    .as_ref()
                    .filter(|pre| pre.as_str() == "xml")
                    .is_none()
                {
                    log::trace!(
                        "`rdf:li` attr isn't a case tag - missing `xml` namespace prefix. \
                        got: `{prefix:?}`, \
                        expected: `Some(\"xml\")`",
                        prefix = keys.prefix
                    );
                    return false;
                }

                // check element
                if keys.local_name.as_str() != "lang" {
                    log::trace!(
                        "`rdf:li` attr isn't a case tag - missing `xml` namespace prefix. \
                        got: `{prefix:?}`, \
                        expected: `Some(\"xml\")`",
                        prefix = keys.prefix
                    );
                    return false;
                }

                true
            });

            if let Some(tag_primitive) = maybe_tag_primitive {
                return Some((
                    tag_primitive,
                    li.to_xmp_element(
                        parse_primitive(li.get_text()?.to_string(), &Prim::Text)
                            .inspect_err(|e| log::error!("Couldn't parse primitive! err: {e}"))
                            .ok()?,
                    )
                    .inspect_err(|e| log::error!("Failed to create `XmpElement`. err: {e}"))
                    .ok()?,
                ));
            }

            None
        })
        .collect();

    // find the default one based on the marker
    let Some(((_chosen_tag_idents, chosen_tag_value), chosen_value)) = parsed_lis
        .iter()
        .find(|((_tag_idents, tag_value), _)| tag_value.as_str() == "x-default")
        .cloned()
    else {
        log::error!("Can't create list of alternatives - no default was found.");
        log::error!("The options found were: {parsed_lis:#?}");
        return Err(XmpParsingError::ArrayAltNoDefault {
            element_name: element.name.clone(),
            alternatives_array: Vec::from_iter(parsed_lis.iter().map(
                |((_tag_idents, tag_value), parsed_li_elem)| {
                    ((*tag_value).clone(), parsed_li_elem.clone())
                },
            )),
        });
    };

    // wrap it all up
    let value = XmpValue::Alternatives {
        chosen: (chosen_tag_value.into(), Box::new(chosen_value)),
        list: parsed_lis
            .into_iter()
            .map(|((_tag_idents, tag_value), value)| (tag_value.clone(), value))
            .collect(),
    };

    element.to_xmp_element(value)
}

/// Parses an element's value as an unordered array of XMP values.
///
/// An unordered array will look like the following:
///
/// ```xml
/// <ns:element>
///      <rdf:Bag>
///          <rdf:li>oswald</rdf:li>
///          <rdf:li>miranda</rdf:li>
///          <rdf:li>natalie</rdf:li>
///          <rdf:li>izzy</rdf:li>
///          <rdf:li> ... </rdf:li>
///      </rdf:Bag>
/// </ns:element>
/// ```
pub fn value_unordered_array(
    element: &Element,
    maybe_ty: Option<&'static Kind>,
) -> XmpElementResult {
    value_array(element, maybe_ty, false)
}

/// Parses an element's value as an ordered array of XMP values.
///
/// An ordered array will look like the following:
///
/// ```xml
/// <ns:element>
///      <rdf:Seq>
///          <rdf:li>value1</rdf:li>
///          <rdf:li>value2</rdf:li>
///          <rdf:li>value3</rdf:li>
///          <rdf:li> ... </rdf:li>
///      </rdf:Seq>
/// </ns:element>
/// ```
pub fn value_ordered_array(element: &Element, maybe_ty: Option<&'static Kind>) -> XmpElementResult {
    value_array(element, maybe_ty, true)
}

/// Parses an element's value as an array.
///
/// `ordered` is `true` if `Kind::OrderedArray`, `false` if
/// `Kind::UnorderedArray`.
fn value_array(
    element: &Element,
    maybe_ty: Option<&'static Kind>,
    ordered: bool,
) -> XmpElementResult {
    let (collection_target, collection_ctor): (&'static str, fn(_) -> _) = match ordered {
        true => ("Seq", XmpValue::OrderedArray),
        false => ("Bag", XmpValue::UnorderedArray),
    };

    // parse out the `rdf:Bag`/`rdf:Seq`
    let collection_elem: &Element = element
        .children
        .iter()
        .flat_map(|cn: &XMLNode| cn.as_element())
        .flat_map(|c: &Element| Some((c, c.namespace.clone()?)))
        .find(|(e, ns)| ns.as_str() == RDF_NAMESPACE && e.name.as_str() == collection_target)
        .map(|(e, _)| e)
        .ok_or(XmpParsingError::ArrayNoInnerCollectionType {
            element_name: element.name.clone(),
            children: element.children.clone(),
        })
        .inspect_err(|e| log::error!("Couldn't find collection container! err: {e}"))?;

    // find the type for each `rdf:li`, if any.
    //
    // note that, if we find a non-array outer type, i.e.
    // `maybe_ty: Some(Struct(...))`, we'll error immediately, as we can't use
    // a schema that isn't an array
    let li_schema = match maybe_ty {
        Some(array_ty) => match array_ty {
            Kind::UnorderedArray(xmp_kind) | Kind::OrderedArray(xmp_kind) => Some(xmp_kind),

            weird_kind => {
                log::error!("Weird schema given to array parser: {weird_kind:#?}");
                return Err(XmpParsingError::ArrayGivenNonArraySchema {
                    element_name: element.name.clone(),
                    weird_schema: weird_kind,
                });
            }
        },
        None => None,
    };

    // grab a list of `rdf:li`
    let lis = collection_elem
        .children
        .iter()
        .flat_map(|cn: &XMLNode| cn.as_element())
        .filter(|maybe_li| {
            // ensure we've got a namespace
            let Some(ref ns) = maybe_li.namespace else {
                log::warn!(
                    "sub-element of `rdf:{collection_target}` was \
                        expected to be `rdf:li`, but had no namespace. \
                        element name: `{}`",
                    &maybe_li.name
                );
                return false;
            };

            // actually check namespace
            ns.as_str() == RDF_NAMESPACE && maybe_li.name.as_str() == "li"
        });

    // parse each `rdf:li` into an `XmpElement`
    let parsed_lis: Vec<_> = lis
        .flat_map(|li| match li_schema {
            Some(ty) => li.value_with_schema(ty),
            None => li.value_generic(),
        })
        .collect();

    // return it as the appropriate array
    element.to_xmp_element(collection_ctor(parsed_lis))
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::{
        xmp::parse_table::XMP_PARSING_MAP,
        xmp::{XmpElement, XmpPrimitive, XmpValue, XmpValueStructField},
    };
    use xmltree::Element;

    use crate::xmp::value::arrays::{
        value_alternatives, value_ordered_array, value_unordered_array,
    };

    /// Ensures we can parse a short array of alternatives.
    #[test]
    fn should_parse_alternatives() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let xml = r#"
<dc:title xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Alt>
        <rdf:li xml:lang="x-default">The Default. Uh... hi!</rdf:li>
        <rdf:li xml:lang="en-US">English (United States). Howdy!</rdf:li>
        <rdf:li xml:lang="de">German. Guten Tag!</rdf:li>
        <rdf:li xml:lang="fr">French. Bonjour !</rdf:li>
        <rdf:li xml:lang="ja">Japanese. こんにちは！</rdf:li>
    </rdf:Alt>
</dc:title>"#;

        let element: Element =
            Element::parse(xml.as_bytes()).expect("valid XML should be parsed by `xmltree`");

        let xmp_element: XmpElement =
            value_alternatives(&element, None).expect("should parse as a list of alternatives.");

        assert_eq!(
            xmp_element,
            XmpElement {
                namespace: "http://purl.org/dc/elements/1.1/".into(),
                prefix: "dc".into(),
                name: "title".into(),
                value: XmpValue::Alternatives {
                    chosen: (
                        "x-default".into(),
                        XmpElement {
                            namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                            prefix: "rdf".into(),
                            name: "li".into(),
                            value: XmpValue::Simple(XmpPrimitive::Text(
                                "The Default. Uh... hi!".into()
                            ))
                        }
                        .into()
                    ),
                    list: vec![
                        (
                            "x-default".into(),
                            XmpElement {
                                namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                                prefix: "rdf".into(),
                                name: "li".into(),
                                value: XmpValue::Simple(XmpPrimitive::Text(
                                    "The Default. Uh... hi!".into()
                                ))
                            }
                        ),
                        (
                            "en-US".into(),
                            XmpElement {
                                namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                                prefix: "rdf".into(),
                                name: "li".into(),
                                value: XmpValue::Simple(XmpPrimitive::Text(
                                    "English (United States). Howdy!".into()
                                ))
                            }
                        ),
                        (
                            "de".into(),
                            XmpElement {
                                namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                                prefix: "rdf".into(),
                                name: "li".into(),
                                value: XmpValue::Simple(XmpPrimitive::Text(
                                    "German. Guten Tag!".into()
                                ))
                            }
                        ),
                        (
                            "fr".into(),
                            XmpElement {
                                namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                                prefix: "rdf".into(),
                                name: "li".into(),
                                value: XmpValue::Simple(XmpPrimitive::Text(
                                    "French. Bonjour !".into()
                                ))
                            }
                        ),
                        (
                            "ja".into(),
                            XmpElement {
                                namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                                prefix: "rdf".into(),
                                name: "li".into(),
                                value: XmpValue::Simple(XmpPrimitive::Text(
                                    "Japanese. こんにちは！".into()
                                ))
                            }
                        ),
                    ]
                }
            },
            "the parsed XMP element should match the expected value."
        );
    }

    /// An ordered array should be parsed in the order it's in.
    #[test]
    fn should_parse_seq_ordered_array() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let xml: &'static str = r#"
<Iptc4xmpExt:rbVertices xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#' xmlns:Iptc4xmpCore='http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/' xmlns:Iptc4xmpExt='http://iptc.org/std/Iptc4xmpExt/2008-02-29/'>
    <rdf:Seq>
        <rdf:li rdf:parseType='Resource'>
            <Iptc4xmpExt:rbX>0.05</Iptc4xmpExt:rbX>
            <Iptc4xmpExt:rbY>0.713</Iptc4xmpExt:rbY>
        </rdf:li>
        <rdf:li rdf:parseType='Resource'>
            <Iptc4xmpExt:rbX>0.148</Iptc4xmpExt:rbX>
            <Iptc4xmpExt:rbY>0.041</Iptc4xmpExt:rbY>
        </rdf:li>
        <rdf:li rdf:parseType='Resource'>
            <Iptc4xmpExt:rbX>0.375</Iptc4xmpExt:rbX>
            <Iptc4xmpExt:rbY>0.863</Iptc4xmpExt:rbY>
        </rdf:li>
    </rdf:Seq>
</Iptc4xmpExt:rbVertices>


            "#;

        let element: Element =
            Element::parse(xml.as_bytes()).expect("`xmltree` should parse XML correctly");

        let xmp: XmpElement = value_ordered_array(&element, None).expect(
            "element contains a valid ordered array, so it should \
                parse correctly",
        );

        assert_eq!(
            xmp,
            XmpElement {
                namespace: "http://iptc.org/std/Iptc4xmpExt/2008-02-29/".into(),
                prefix: "Iptc4xmpExt".into(),
                name: "rbVertices".into(),
                value: XmpValue::OrderedArray(vec![
                    XmpElement {
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        name: "li".into(),
                        value: XmpValue::Struct(vec![
                            XmpValueStructField::Value {
                                ident: "rbX".into(),
                                namespace: Some(
                                    "http://iptc.org/std/Iptc4xmpExt/2008-02-29/".into()
                                ),
                                value: XmpValue::Simple(XmpPrimitive::Text("0.05".into()))
                            },
                            XmpValueStructField::Value {
                                ident: "rbY".into(),
                                namespace: Some(
                                    "http://iptc.org/std/Iptc4xmpExt/2008-02-29/".into()
                                ),
                                value: XmpValue::Simple(XmpPrimitive::Text("0.713".into()))
                            },
                        ]),
                    },
                    XmpElement {
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        name: "li".into(),
                        value: XmpValue::Struct(vec![
                            XmpValueStructField::Value {
                                ident: "rbX".into(),
                                namespace: Some(
                                    "http://iptc.org/std/Iptc4xmpExt/2008-02-29/".into()
                                ),
                                value: XmpValue::Simple(XmpPrimitive::Text("0.148".into()))
                            },
                            XmpValueStructField::Value {
                                ident: "rbY".into(),
                                namespace: Some(
                                    "http://iptc.org/std/Iptc4xmpExt/2008-02-29/".into()
                                ),
                                value: XmpValue::Simple(XmpPrimitive::Text("0.041".into()))
                            },
                        ]),
                    },
                    XmpElement {
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        name: "li".into(),
                        value: XmpValue::Struct(vec![
                            XmpValueStructField::Value {
                                ident: "rbX".into(),
                                namespace: Some(
                                    "http://iptc.org/std/Iptc4xmpExt/2008-02-29/".into()
                                ),
                                value: XmpValue::Simple(XmpPrimitive::Text("0.375".into()))
                            },
                            XmpValueStructField::Value {
                                ident: "rbY".into(),
                                namespace: Some(
                                    "http://iptc.org/std/Iptc4xmpExt/2008-02-29/".into()
                                ),
                                value: XmpValue::Simple(XmpPrimitive::Text("0.863".into()))
                            },
                        ]),
                    }
                ])
            }
        );
    }

    /// This unordered array should parse correctly.
    #[test]
    fn should_parse_bag_unordered_array() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let xml: &'static str = r#"

 <Iptc4xmpCore:Scene xmlns:Iptc4xmpCore="http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
      <rdf:Bag>
        <rdf:li>011221</rdf:li>
           <rdf:li>012221</rdf:li>
 </rdf:Bag>
      </Iptc4xmpCore:Scene>"#;

        let element: Element =
            Element::parse(xml.as_bytes()).expect("`xmltree` should parse XML correctly");

        let schema = XMP_PARSING_MAP
            .get(&("http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/", "Scene"))
            .unwrap();

        let xmp: XmpElement = value_unordered_array(&element, Some(schema))
            .expect("xml contains valid unordered array");

        assert_eq!(
            xmp,
            XmpElement {
                namespace: "http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/".into(),
                prefix: "Iptc4xmpCore".into(),
                name: "Scene".into(),
                value: XmpValue::UnorderedArray(vec![
                    XmpElement {
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        name: "li".into(),
                        value: XmpValue::Simple(XmpPrimitive::Text("011221".into()))
                    },
                    XmpElement {
                        namespace: "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
                        prefix: "rdf".into(),
                        name: "li".into(),
                        value: XmpValue::Simple(XmpPrimitive::Text("012221".into()))
                    },
                ])
            }
        );
    }
}

use raves_metadata_types::{
    xmp::{XmpElement, XmpPrimitive, XmpValue, XmpValueStructField},
    xmp_parsing_types::{XmpKind as Kind, XmpKindStructField as Field},
};
use xmltree::Element;

use crate::xmp::{
    RDF_NAMESPACE,
    error::XmpParsingError,
    heuristics::XmpElementHeuristicsExt as _,
    value::{XmpElementExt, prims::parse_primitive},
};

/// Parses an element as a struct with fields.
///
/// Each field is parsed recursively.
pub fn value_struct<'elem>(
    element: &'elem Element,
    maybe_schema: Option<&'static Kind>,
) -> Result<XmpElement<'elem>, XmpParsingError<'elem>> {
    // helper closure: grab a field from the schema
    let get_field_info = |field_ns: &Option<&str>, field_name: &str| -> Option<&Field> {
        let Some(Kind::Struct(schema_fields)) = maybe_schema else {
            log::trace!("No field schema given. Setting `schema` to `None`.");
            return None;
        };

        log::trace!("A schema was given! Checking field info...");
        schema_fields.iter().find(|field| {
            field.ident.name() == field_name
                && if let Some(string_ns) = field_ns {
                    log::trace!("Field namespace found! Will return based on namespace eq...");
                    field.ident.ns() == Some(string_ns)
                } else {
                    field.ident.ns().is_none()
                }
        })
    };

    // parse attribute fields, if any
    let attr_fields = element.attributes.iter().flat_map(|(keys, value)| {
        // unwrap the namespace + name
        let (ns, name) = (&keys.namespace_ref(), &keys.local_name);

        // skip the `rdf:parseType` and `rdf:about` fields
        if (name == "parseType" || name == "about")
            && ns.is_some_and(|inner_ns: &str| inner_ns == RDF_NAMESPACE)
        {
            log::trace!("Won't make a field from `rdf:parseType` attribute.");
            return None;
        }

        // we can parse for a specific value if our schema knows what to look
        // for.
        //
        // however, we can only parse primitives, so complain if it asks us to
        // do something else
        log::trace!("Getting field info on struct field: `{name}`");
        if let Some(field_info) = get_field_info(ns, name) {
            // check that it's a primitive
            let Kind::Simple(prim) = field_info.ty else {
                log::error!(
                    "Found attribute on struct, but schema asked for non-primitive type: {:#?}",
                    field_info.ty
                );
                return None;
            };

            // parse the primitive
            log::trace!(
                "Struct field `{name}` was in the schema! \
                Constructing accordingly..."
            );
            return Some(XmpValueStructField::Value {
                ident: field_info.ident.name().into(),
                namespace: field_info.ident.ns().map(|ns| ns.into()),
                value: parse_primitive(value.into(), prim)
                    .inspect_err(|e| {
                        log::error!(
                            "On attribute field `{name}`, failed to parse primitive. err: {e}"
                        )
                    })
                    .ok()?,
            });
        }

        // alright, this isn't in the schema.
        //
        // that makes things easy!
        log::trace!(
            "Struct field `{name}` was not in the schema! \
            Constructing text value..."
        );
        Some(XmpValueStructField::Value {
            ident: name.into(),
            namespace: (*ns).map(|n| n.into()),
            value: XmpValue::Simple(XmpPrimitive::Text(value.into())),
        })
    });

    // now, let's check if we're a container, or if we have a container.
    //
    // if we've got neither, then we're done here.
    let Some(fields_container) = (match element.attributes.iter().any(|(attr_keys, attr_value)| {
        attr_keys.local_name == "parseType"
            && attr_keys
                .namespace_ref()
                .is_some_and(|inner_ns| inner_ns == RDF_NAMESPACE)
            && attr_value == "Resource"
    }) {
        true => Some(element),
        false => element
            .get_child("Description")
            .filter(|desc_elem| desc_elem.is_rdf_description()),
    }) else {
        // if we're not a container, and we don't have a container, then we've
        // nothing left to parse..!
        log::trace!(
            "We aren't a container, and don't have a container! \
            Stopping struct `{struct_name}` here...",
            struct_name = &element.name
        );
        return element.to_xmp_element(XmpValue::Struct(attr_fields.collect()));
    };

    // alright, we do have fields to parse.
    //
    // let's handle those!
    let inner_fields = fields_container
        .children
        .iter()
        .flat_map(|c| c.as_element())
        .flat_map(|c| {
            log::trace!(
                "Parsing inner field `{inner_field_name}` on struct `{struct_name}`...",
                inner_field_name = &c.name,
                struct_name = &element.name
            );

            value_struct_field(
                c,
                get_field_info(
                    &match &c.namespace {
                        Some(ns) => Some(ns.as_str()),
                        None => None,
                    },
                    &c.name,
                ),
            )
        });

    // pop it all into a new XmpValue
    element.to_xmp_element(XmpValue::Struct(attr_fields.chain(inner_fields).collect()))
}

/// Parses an element as a struct field.
pub fn value_struct_field<'xml>(
    element: &'xml Element,
    maybe_field_kind: Option<&'static Field>,
) -> Option<XmpValueStructField<'xml>> {
    // if we know the field we're workin with, we can apply its schema.
    //
    // otherwise, we'll have to guess carefully...
    let (ident, namespace, element) = match maybe_field_kind {
        // get em from the schema
        Some(field_kind) => (
            field_kind.ident.name().into(),
            field_kind.ident.ns().map(|s| s.into()),
            element
                .value_with_schema(field_kind.ty)
                .inspect_err(|e| log::error!("Field with known schema failed to parse! err: {e}"))
                .ok()?,
        ),

        // get em from the field
        None => (
            element.name.clone().into(),
            element.namespace.clone().map(|s| s.into()),
            match element.value_generic() {
                Ok(s) => s,
                Err(e) => {
                    log::trace!("Parsing value generically failed! err: {e}");
                    return None;
                }
            },
        ),
    };

    Some(match element.value {
        XmpValue::Simple(_) => XmpValueStructField::Value {
            ident,
            namespace,
            value: element.value,
        },
        _ => XmpValueStructField::Element {
            ident,
            namespace,
            element,
        },
    })
}

#[cfg(test)]
mod tests {
    use crate::xmp::{Xmp, XmpDocument};

    /// The parser should be able to handle several different layouts of
    /// structs.
    ///
    /// Adobe's XMP specification lays out an example similar to the one below:
    #[test]
    fn several_struct_types() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let xmp: Xmp = Xmp::new(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
      <rdf:Description rdf:about="" xmlns:ns="ns:myName/">

          <!-- struct 1: regular syntax -->
          <ns:Struct1>
              <rdf:Description>
                  <ns:Field1>value1</ns:Field1>
                  <ns:Field2>value2</ns:Field2>
              </rdf:Description>
          </ns:Struct1>

          <!-- struct 2: condensed (no inner rdf:Description tag) -->
          <ns:Struct2 rdf:parseType="Resource">
              <ns:Field1>value1</ns:Field1>
              <ns:Field2>value2</ns:Field2>
          </ns:Struct2>

          <!--- struct 3: fields as desc attributes (shorthand) -->
          <ns:Struct3>
              <rdf:Description ns:Field1="value1" ns:Field2="value2"/>
          </ns:Struct3>

          <!--- struct 4: fields as self atttributes (shorthand) -->
          <ns:Struct4 ns:Field1="value1" ns:Field2="value2"/>

          <!--- struct 5 -->
          <ns:Struct5>
              <rdf:Description ns:Field1="value1">
                  <ns:Field2>value2</ns:Field2>
              </rdf:Description>
          </ns:Struct5>
      </rdf:Description>
    </rdf:RDF>"#,
        )
        .unwrap();

        let parsed: XmpDocument = xmp
            .parse()
            .expect("`raves_metadata` should parse the description correctly");
        assert_eq!(parsed.values_ref().len(), 5);
    }
}

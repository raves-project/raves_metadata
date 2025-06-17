use raves_metadata::xmp::{
    Xmp, XmpDocument,
    types::{XmpPrimitive, XmpValue, XmpValueStructField},
};

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

/// Checks that a known struct type parses correctly.
#[test]
fn known_struct_type() {
    _ = env_logger::builder()
        .filter_level(log::LevelFilter::max())
        .format_file(true)
        .format_line_number(true)
        .try_init();

    // edited sample from the adobe spec (p. 19)
    let xmp: Xmp = Xmp::new(
        r#"
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
        <rdf:Description>
            <xmpTPg:MaxPageSize rdf:about="" xmlns:xmpTPg="http://ns.adobe.com/xap/1.0/t/pg/">
                <rdf:Description xmlns:stDim="http://ns.adobe.com/xap/1.0/sType/Dimensions#">
                    <stDim:w>4</stDim:w>
                    <stDim:h>3</stDim:h>
                    <stDim:unit>inch</stDim:unit>
                </rdf:Description>
            </xmpTPg:MaxPageSize>
        </rdf:Description>
    </rdf:RDF>"#,
    )
    .expect("`xmltree` should parse the XML correctly");

    // run our parser over it
    let parsed: XmpDocument = xmp
        .parse()
        .expect("`raves_metadata` can parse a known struct");

    // try grabbing it as a struct
    let maybe_struct_val = parsed.values_ref().first().unwrap();
    let XmpValue::Struct(mut s) = maybe_struct_val.clone().value else {
        panic!("not a struct! got: {maybe_struct_val:#?}");
    };

    s.sort_by_key(|field| field.ident());

    // ensure the values are correct
    assert_eq!(s, {
        let mut v = vec![
            XmpValueStructField::Element {
                ident: "w".into(),
                namespace: Some(r"http://ns.adobe.com/xap/1.0/sType/Dimensions#".into()),
                element: raves_metadata_types::xmp::XmpElement {
                    namespace: "http://ns.adobe.com/xap/1.0/sType/Dimensions#".into(),
                    prefix: "stDim".into(),
                    name: "w".into(),
                    value: XmpValue::Simple(XmpPrimitive::Real(4.0)),
                },
            },
            XmpValueStructField::Element {
                ident: "h".into(),
                namespace: Some(r"http://ns.adobe.com/xap/1.0/sType/Dimensions#".into()),
                element: raves_metadata_types::xmp::XmpElement {
                    namespace: "http://ns.adobe.com/xap/1.0/sType/Dimensions#".into(),
                    prefix: "stDim".into(),
                    name: "h".into(),
                    value: XmpValue::Simple(XmpPrimitive::Real(3.0)),
                },
            },
            XmpValueStructField::Element {
                ident: "unit".into(),
                namespace: Some(r"http://ns.adobe.com/xap/1.0/sType/Dimensions#".into()),
                element: raves_metadata_types::xmp::XmpElement {
                    namespace: "http://ns.adobe.com/xap/1.0/sType/Dimensions#".into(),
                    prefix: "stDim".into(),
                    name: "unit".into(),
                    value: XmpValue::Simple(XmpPrimitive::Text("inch".into())),
                },
            },
        ];

        v.sort_by_key(|field| field.ident());
        v
    });
}

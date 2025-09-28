use raves_metadata::xmp::{
    Xmp,
    types::{XmpPrimitive, XmpValue, XmpValueStructField},
};

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
    .expect("`raves_metadata` can parse a known struct");

    // try grabbing it as a struct
    let maybe_struct_val = xmp.document().values_ref().first().unwrap();
    let XmpValue::Struct(mut s) = maybe_struct_val.clone().value else {
        panic!("not a struct! got: {maybe_struct_val:#?}");
    };

    s.sort_by_key(|field| field.ident().to_string());

    // ensure the values are correct
    assert_eq!(s, {
        let mut v = vec![
            XmpValueStructField::Value {
                ident: "w".into(),
                namespace: Some(r"http://ns.adobe.com/xap/1.0/sType/Dimensions#".into()),
                value: XmpValue::Simple(XmpPrimitive::Real(4.0)),
            },
            XmpValueStructField::Value {
                ident: "h".into(),
                namespace: Some(r"http://ns.adobe.com/xap/1.0/sType/Dimensions#".into()),
                value: XmpValue::Simple(XmpPrimitive::Real(3.0)),
            },
            XmpValueStructField::Value {
                ident: "unit".into(),
                namespace: Some(r"http://ns.adobe.com/xap/1.0/sType/Dimensions#".into()),
                value: XmpValue::Simple(XmpPrimitive::Text("inch".into())),
            },
        ];

        v.sort_by_key(|field| field.ident().to_string());
        v
    });
}

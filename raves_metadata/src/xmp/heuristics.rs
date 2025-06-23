//! This module assists in guessing the type of an XML element.
//!
//! We use the functions defined here when an element isn't in the parse table,
//! so we don't know what types it's intended to hold.

use super::RDF_NAMESPACE;

pub trait XmpElementHeuristicsExt {
    fn is_struct(&self) -> bool;
    fn is_rdf_description(&self) -> bool;
    fn has_collection(&self) -> Option<CollectionKind>;
}

impl XmpElementHeuristicsExt for xmltree::Element {
    fn is_rdf_description(&self) -> bool {
        // all of em have namespaces
        let Some(ref ns) = self.namespace else {
            return false;
        };

        // ns should be the typical rdf one
        if ns != RDF_NAMESPACE {
            return false;
        }

        // name should match
        if self.name != "Description" {
            return false;
        }

        true
    }

    /// Determines whether or not `self` is a struct.
    fn is_struct(&self) -> bool {
        // if we have the `rdf:parseType="Resource"`, we must be a struct.
        if self.attributes.iter().any(|(attr_keys, attr_value)| {
            attr_keys.local_name == "parseType"
                && attr_keys
                    .namespace_ref()
                    .is_some_and(|inner_ns| inner_ns == RDF_NAMESPACE)
                && attr_value == "Resource"
        }) {
            return true;
        }

        // if any of our children are `rdf:Description`, we carry fields and
        // must be a struct
        if self
            .children
            .iter()
            .flat_map(|c| c.as_element())
            .any(|c| c.is_rdf_description())
        {
            return true;
        }

        // if we have fields and no sub-elements, we're considered a struct
        if self.children.is_empty() // we have no sub-elements
        && self.attributes.iter().any(|(key, _val)| {
            // (any key) has (any namespace) except that of RDF.
            //
            // this allows us to avoid finding "structs" that are actually
            // blank keys with no attached fields/value.
            key.namespace_ref().is_none_or(|ns| ns != RDF_NAMESPACE)
        }) {
            return true;
        }

        false
    }

    fn has_collection(&self) -> Option<CollectionKind> {
        self.children
            .iter()
            .flat_map(|c| c.as_element())
            .filter_map(|c| match c.name.as_str() {
                "Alt" => Some((c, CollectionKind::Alternatives)),
                "Bag" => Some((c, CollectionKind::Unordered)),
                "Seq" => Some((c, CollectionKind::Ordered)),
                _ => None,
            })
            .find(|(collection_elem, _)| {
                // we require a namespace
                if let Some(ref ns) = collection_elem.namespace {
                    // and the namespace should be `rdf`
                    if ns == RDF_NAMESPACE {
                        return true;
                    }
                };

                false
            })
            .map(|(_, kind)| kind)
    }
}

/// The kind of collection we've detected.
#[derive(Debug, PartialEq)]
pub enum CollectionKind {
    Alternatives,
    Unordered,
    Ordered,
}

#[cfg(test)]
mod tests {
    use crate::xmp::heuristics::{CollectionKind, XmpElementHeuristicsExt};
    use xmltree::Element;

    /// An `rdf:Description` element must meet these three requirements:
    ///
    /// 1. it must have a namespace.
    /// 2. the namespace's value should match that of `rdf`.
    /// 3. its element name must be `Description`.
    ///
    /// We should reject potential "descriptions" that don't follow all of
    /// these.
    #[test]
    fn check_is_rdf_description() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        // blank descriptions are fine!
        {
            let element = Element::parse(
                r#"<rdf:Description xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"></rdf:Description>"#
                .as_bytes(),
            )
            .expect("xmltree parsing");

            assert!(
                element.is_rdf_description(),
                "blank description should be a description"
            );
        }

        // descriptions with stuff are, too...
        {
            let element = Element::parse(
                r#"
<rdf:Description xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <Struct4 Field1="value1" Field2="value2" />
</rdf:Description>"#
                    .as_bytes(),
            )
            .expect("xmltree parsing");

            assert!(
                element.is_rdf_description(),
                "description with a struct should be a description"
            );
        }

        // alright, this one won't have a namespace.
        //
        // that's wrong!
        {
            let element = Element::parse(
                r#"
<Description xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <Struct4 Field1="value1" Field2="value2" />
</Description>"#
                    .as_bytes(),
            )
            .expect("xmltree parsing");

            assert!(
                !element.is_rdf_description(),
                "just `<Description>` shouldn't be considered a description."
            );
        }

        // here, we fail since the namespace's value should match the typical
        // URL, not my GitHub.
        {
            let element = Element::parse(
                r#"
<rdf:Description xmlns:rdf="https://github.com/onkoe">
    <Struct4 Field1="value1" Field2="value2" />
</rdf:Description>"#
                    .as_bytes(),
            )
            .expect("xmltree parsing");

            assert!(
                !element.is_rdf_description(),
                "`<rdf:Description>` shouldn't be considered a description \
                unless it has the correct namespace URL."
            );
        }

        // while we use `rde:Description` here, that's totally fine, so long as
        // the namespace URL matches
        {
            let element = Element::parse(
                r#"
<rde:Description xmlns:rde="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <Struct4 Field1="value1" Field2="value2" />
</rde:Description>"#
                    .as_bytes(),
            )
            .expect("xmltree parsing");

            assert!(
                element.is_rdf_description(),
                "`<rde:Description>` should be accepted with a matching \
                namespace URL"
            );
        }

        // while we use `rde:Description` here, that's totally fine, so long as
        // the namespace URL matches
        {
            let element = Element::parse(
                r#"
<rde:Description xmlns:rde="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <Struct4 Field1="value1" Field2="value2" />
</rde:Description>"#
                    .as_bytes(),
            )
            .expect("xmltree parsing");

            assert!(
                element.is_rdf_description(),
                "`<rde:Description>` should be accepted with a matching \
                namespace URL"
            );
        }

        // `rdf:description` doesn't match the RDF specification - it must be
        // capitalized.
        //
        // see: https://www.w3.org/TR/rdf-syntax-grammar/#section-Namespace:~:text=RDF-,Description
        {
            let element = Element::parse(
                r#"
<rdf:description xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <Struct4 Field1="value1" Field2="value2" />
</rdf:description>"#
                    .as_bytes(),
            )
            .expect("xmltree parsing");

            assert!(
                !element.is_rdf_description(),
                "`rdf:description` must be capitalized, like `rdf:Description`"
            );
        }
    }

    /// `is_struct` should find a struct for each of the structs in the given
    /// XML file.
    #[test]
    fn is_struct_stress_test_all_structs() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let xml = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
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
      <ns:Struct4 ns:Field1="value1" ns:Field2="value2" />

      <!--- struct 5 -->
      <ns:Struct5>
          <rdf:Description ns:Field1="value1">
              <ns:Field2>value2</ns:Field2>
          </rdf:Description>
      </ns:Struct5>
  </rdf:Description>
</rdf:RDF>"#;

        let mut rdf_rdf = Element::parse(xml.as_bytes()).expect("find rdf:RDF");
        let rdf_description = rdf_rdf
            .take_child("Description")
            .expect("find rdf:Description");
        let desc_children = rdf_description
            .children
            .iter()
            .flat_map(|c| c.as_element())
            .collect::<Vec<_>>();

        assert_eq!(
            desc_children.len(),
            5_usize,
            "should be five desc sub-elements"
        );

        for c in desc_children {
            assert!(c.is_struct(), "struct `{}` should be a struct!", c.name);
        }
    }

    /// `is_struct` shouldn't think any of these are structs.
    #[test]
    fn is_struct_stress_test_not_structs() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        let xml = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about="" xmlns:ns="ns:myName/">

        <!-- not struct 1: non-rdf inner description

        if a struct has sub-elements, and lacks an `rdf:parseType`, it must
        have a field `rdf:Description`. however, this one contains an
        `ns:Description`, which isn't the same.
        -->
        <ns:NotAStruct1>
                <ns:Field1>value1</ns:Field1>
                <ns:Field2>value2</ns:Field2>
                <ns:Description>im not an `rdf:Description` btw lol</ns:Description>
        </ns:NotAStruct1>

        <!-- not struct 2: parseType has value of `NotAResource`

        for structs defined by an `rdf:parseType` attribute, the attribute's
        value must be `Resource`.

        this one has `NotAResource`, which should fail the struct.
        -->
        <ns:NotAStruct2 rdf:parseType="NotAResource">
            <ns:Field1>value1</ns:Field1>
            <ns:Field2>value2</ns:Field2>
        </ns:NotAStruct2>

        <!-- not struct 3: no real fields

        structs must have actual fields. however, this struct only specifies
        `rdf` parsing instructions.

        those aren't fields, so this would be a blank struct, which doesn't
        appear to be an allowed state in XMP.
        -->
        <ns:NotAStruct3
            rdf:parseInfo="parse RDF with this one simple trick"
            rdf:specialGift="for the first 100 who sign up"
        />


    </rdf:Description>
    </rdf:RDF>"#;

        let mut rdf_rdf = Element::parse(xml.as_bytes()).expect("find rdf:RDF");
        let rdf_description = rdf_rdf
            .take_child("Description")
            .expect("find rdf:Description");
        let desc_children = rdf_description
            .children
            .iter()
            .flat_map(|c| c.as_element())
            .collect::<Vec<_>>();

        assert_eq!(desc_children.len(), 3_usize);

        for c in desc_children {
            assert!(!c.is_struct(), "struct `{}` isn't a struct.", c.name);
        }
    }

    /// `has_collection` should properly find `rdf:Alt`, `rdf:Bag`, and
    /// `rdf:Seq`.
    #[test]
    fn valid_containers_in_has_collection() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        for container_element_name in ["Alt", "Bag", "Seq"] {
            let xml = format!(
                r#"
<my_ns:myElement xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:my_ns="https://github.com/onkoe">
    <rdf:{container_element_name}>
        <rdf:li>Some list sub-elements in here. Note that `rdf:Alt` needs more info than this.</rdf:li>
    </rdf:{container_element_name}>
</my_ns:myElement>"#
            );

            let element =
                Element::parse(xml.as_bytes()).expect("`xmltree` xml parsing should work");

            assert_eq!(
                element.has_collection().expect("should have a collection"),
                match container_element_name {
                    "Alt" => CollectionKind::Alternatives,
                    "Bag" => CollectionKind::Unordered,
                    "Seq" => CollectionKind::Ordered,
                    _ => unreachable!(),
                }
            )
        }
    }

    /// On the other hand, `rdf:alt`, `rdf:bag`, and `rdf:seq` should all fail.
    ///
    /// Other text should also fail.
    #[test]
    fn invalid_containers_should_fail_in_has_collection() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .format_file(true)
            .format_line_number(true)
            .try_init();

        for container_element_name in ["alt", "bag", "seq", "other"] {
            let xml = format!(
                r#"
<my_ns:myElement xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:my_ns="https://github.com/onkoe">
    <rdf:{container_element_name}>
        <li>list element</li>
    </rdf:{container_element_name}>
</my_ns:myElement>"#
            );

            let element =
                Element::parse(xml.as_bytes()).expect("`xmltree` xml parsing should work");

            assert!(element.has_collection().is_none())
        }
    }
}

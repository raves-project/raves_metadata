//! Some formats use XMP to store their IPTC.
//!
//! In this case, we need a parser to do some bare minimum XMP parsing: we
//! simply disregard anything that isn't the IPTC information.

use crate::{Iptc, error::XmpError};

use raves_metadata_types::iptc::{
    IptcKey as K, IptcKeyValue as Kv,
    structs::{
        ArtworkOrObject, CopyrightOwner, CreatorContactInfo, CvTerm, EmbdEncRightsExpr, Entity,
        EntityWRole, ImageCreator, ImageRegion, ImageSupplier, Licensor, LinkedEncRightsExpr,
        Location, PersonWDetails, ProductWGtin, RegionBoundary, RegionBoundaryPoint, RegistryEntry,
    },
};
use xmltree::Element;

pub fn parse_xmp_for_iptc(raw_xmp: &str) -> Result<Iptc, XmpError> {
    // shove the XMP data into an XML parser...
    let document = match Element::parse(raw_xmp.as_bytes()) {
        Ok(elem) => elem,
        Err(e) => todo!("map into XmpError after we get good err types. err: {e}"),
    };

    // here's our IPTC pairs list.
    //
    // we'll continue adding to this as we parse the XMP
    let mut pairs: Vec<Kv> = Vec::new();

    // TODO: recursively scan for the `rdf:RDF` element on all document children.
    //
    // if found, we'll use that to continue. otherwise, we stop parsing and
    // return an error
    //
    //TODO! below, is a hacky way to grab `rdf:RDF`, but we should really look
    //more closely at the document to avoid missing it.
    //
    //some popular examples don't use the standard-compliant layout...

    // grab the `rdf:Description`(s) for IPTC parsing.
    //
    // since we're currently only parsing internal IPTC, we can safely grab the
    // three RDF nodes and just parse the last one.
    //
    // note that `x:xmpmeta` is an optional element, so we'll only try to parse
    // it out. it's sometimes the "parent element" of the whole document, too.
    let parent = if document.prefixed_name() == "x:xmp_meta" {
        document.prefixed_child("x:xmp_meta").unwrap_or(&document)
    } else {
        &document
    };
    log::debug!("found parent element: {}", parent.prefixed_name());

    let Some(rdf) = parent.prefixed_child("rdf:RDF") else {
        todo!("make this into an error: \"required rdf:RDF element wasn't found in XMP\"");
    };
    log::debug!("found `rdf:RDF`: {}", parent.prefixed_name());

    // there can be multiple RDF descriptions, so we'll grab all of them that
    // are inner nodes!
    let rdf_descriptions = rdf
        .children
        .iter()
        .flat_map(|inner| inner.as_element())
        .filter(|elem| elem.prefixed_name() == "rdf:Description");

    log::debug!(
        "found {} `rdf:Description` tags.",
        rdf_descriptions.clone().count()
    );

    // now, we can parse each of the `rdf:Description` subnodes to grab any
    // potential IPTC info
    for rdf_description in rdf_descriptions {
        // first, parse its attributes for builtin IPTC primitives
        for (attr_key, attr_value) in &rdf_description.attributes {
            log::debug!(
                "parsing attribute: `{}:{}` = `{attr_value}` in RDF description",
                attr_key.prefix_ref().unwrap_or(""),
                attr_key.local_name
            );

            // it must have a prefix; otherwise, we'll skip it
            let Some(attr_prefix) = attr_key.prefix_ref() else {
                log::debug!("skipping attribute `{attr_key}`, as it has no prefix");
                continue;
            };

            // grab the attribute's associated key type
            let prefixed_key: String = format!("{}:{}", attr_prefix, attr_key.local_name);
            let Some(iptc_ty) = K::from_xmp_id(&prefixed_key) else {
                log::debug!("skipping attribute `{prefixed_key}`, as it isn't an IPTC key");
                continue;
            };

            // combine it with its value
            if let Some(iptc_pair) = iptc_pair_from_simple_text_value(iptc_ty, attr_value) {
                pairs.push(iptc_pair);
            }
        }

        // then, its subnodes
        for xml_node in &rdf_description.children {
            // we only care about the children that are elements
            let Some(element) = xml_node.as_element() else {
                continue;
            };

            // we only care about IPTC pairs.
            //
            // got anything else? disregard it.
            let Some(key) = K::from_xmp_id(&element.name) else {
                continue;
            };

            // first, let's check for elements that are simple.
            //
            // they only contain their data as text, possibly inside an inner
            // `rdf:Description` element (though not always)
            if let Some(text) = element.get_text().or_else(|| {
                element
                    .prefixed_child("rdf::Description")
                    .and_then(|desc| desc.get_text())
            }) {
                // map the text to the element's known type
                if let Some(iptc_pair) = iptc_pair_from_simple_text_value(key, text) {
                    pairs.push(iptc_pair);
                    continue;
                }
            }

            // now, we can map any "vec" types (elements w/ lists)
            if key.has_vec_ty() {
                if let Some(iptc_pair) = parse_vec_types(key, element) {
                    pairs.push(iptc_pair);
                    continue;
                }
            }

            // finally, we can parse the struct types.
            //
            // ...so. struct types have two ways to exist:
            //
            // 1. shorthand: `<structname field1="val1" field2="val2" ... />`
            // 2. regular: `<structname> <field1> val </field1> </structname>`
            //
            // we'll need to parse each case separately to do this right.
            if key.has_struct_ty() {
                match element.children.is_empty() {
                    // perform shorthand parsing
                    true => {
                        if let Some(pair) = parse_shorthand_struct_types(key, element) {
                            pairs.push(pair);
                            continue;
                        }
                    }

                    // do regular parsing
                    false => {
                        if let Some(pair) = parse_regular_struct_types(key, element) {
                            pairs.push(pair);
                            continue;
                        }
                    }
                }
            }

            // if we're here, we didn't find a pair for this element yet.
        }
    }

    Ok(Iptc { pairs })
}

/// Maps a simple text value into an IPTC pair, if possible.
fn iptc_pair_from_simple_text_value(key: K, value: impl Into<String>) -> Option<Kv> {
    let value: String = Into::<String>::into(value);

    // map the Key into a KeyValue using the data we got.
    //
    // we can only map stuff that's a primitive, though! ignore anything that
    // isn't...
    Some(match key {
        // these are all the stringy ones
        K::AltTextAccessibility => Kv::AltTextAccessibility(value),
        K::CityLegacy => Kv::CityLegacy(value),
        K::CopyrightNotice => Kv::CopyrightNotice(value),
        K::CountryLegacy => Kv::CountryLegacy(value),
        K::CountryCodeLegacy => Kv::CountryCodeLegacy(value),
        K::CreatorsJobtitle => Kv::CreatorsJobtitle(value),
        K::CreditLine => Kv::CreditLine(value),
        K::DateCreated => Kv::DateCreated(value),
        K::Description => Kv::Description(value),
        K::DescriptionWriter => Kv::DescriptionWriter(value),
        K::ExtendedDescriptionAccessibility => Kv::ExtendedDescriptionAccessibility(value),
        K::Headline => Kv::Headline(value),
        K::Instructions => Kv::Instructions(value),
        K::IntellectualGenreLegacy => Kv::IntellectualGenreLegacy(value),
        K::JobId => Kv::JobId(value),
        K::ProvinceOrStateLegacy => Kv::ProvinceOrStateLegacy(value),
        K::RightsUsageTerms => Kv::RightsUsageTerms(value),
        K::SourceSupplyChain => Kv::SourceSupplyChain(value),
        K::SublocationLegacy => Kv::SublocationLegacy(value),
        K::Title => Kv::Title(value),
        K::AdditionalModelInformation => Kv::AdditionalModelInformation(value),
        K::DataMining => Kv::DataMining(value),
        K::OtherConstraints => Kv::OtherConstraints(value),
        K::DigitalImageGuid => Kv::DigitalImageGuid(value),
        K::DigitalSourceType => Kv::DigitalSourceType(value),
        K::EventName => Kv::EventName(value),
        K::ImageSupplierImageId => Kv::ImageSupplierImageId(value),
        K::MinorModelAgeDisclosure => Kv::MinorModelAgeDisclosure(value),
        K::ModelReleaseStatus => Kv::ModelReleaseStatus(value),
        K::PropertyReleaseStatus => Kv::PropertyReleaseStatus(value),
        K::WebStatementOfRights => Kv::WebStatementOfRights(value),

        // for the numeric ones, we just parse out a number and try to use it
        K::ImageRating => Kv::ImageRating(value.parse::<f64>().ok()?),
        K::MaxAvailHeight => Kv::MaxAvailHeight(value.parse::<i64>().ok()?),
        K::MaxAvailWidth => Kv::MaxAvailWidth(value.parse::<i64>().ok()?),

        // ignore vec types
        K::Creator
        | K::Keywords
        | K::SceneCode
        | K::SubjectCodeLegacy
        | K::CodeOfOrganisationFeaturedInTheImage
        | K::EventIdentifier
        | K::ModelReleaseId
        | K::NameOfOrganisationFeaturedInTheImage
        | K::PersonShownInTheImage
        | K::PropertyReleaseId
        | K::ModelAge => return None,

        // ignore struct types
        K::CreatorsContactInfo
        | K::ArtworkOrObjectInTheImage
        | K::Contributor
        | K::CopyrightOwner
        | K::CvTermAboutImage
        | K::EmbeddedEncodedRightsExpression
        | K::Genre
        | K::ImageCreator
        | K::ImageRegion
        | K::ImageRegistryEntry
        | K::ImageSupplier
        | K::Licensor
        | K::LinkedEncodedRightsExpression
        | K::LocationCreated
        | K::LocationShownInTheImage
        | K::PersonShownInTheImageWithDetails
        | K::ProductShownInTheImage => return None,
    })
}

/// Parses a vector type that's in XMP format.
///
/// These typically use the `rdf:{Alt, Bag, Seq}` elements.
fn parse_vec_types(key: K, element: &Element) -> Option<Kv> {
    // sometimes, certain serializers will throw in an extra
    // `rdf::Description`.
    //
    // if it's available, we'll grab that here to use it as the list's parent
    let parent = match element
        .children
        .iter()
        .flat_map(|c| c.as_element())
        .find(|elem| {
            elem.prefix
                .clone()
                .is_some_and(|ref p| p == "rdf" && elem.name == "Description")
        }) {
        Some(desc) => desc,
        None => element,
    };

    // we'll only search if there's an rdf list element.
    //
    // that avoids throwing random shit at the `get_list` helper
    let list_elem = parent
        .children
        .iter()
        .flat_map(|c| c.as_element())
        .find(|elem| elem.is_collection_element())?;

    // we can grab the list now before using it.
    let list = list_elem.get_list();

    // finally, map the list into values for each of these "vec-typed" pairs
    Some(match key {
        K::Creator => Kv::Creator(list),
        K::Keywords => Kv::Keywords(list),
        K::SceneCode => Kv::SceneCode(list),
        K::SubjectCodeLegacy => Kv::SubjectCodeLegacy(list),
        K::CodeOfOrganisationFeaturedInTheImage => Kv::CodeOfOrganisationFeaturedInTheImage(list),
        K::EventIdentifier => Kv::EventIdentifier(list),
        K::ModelReleaseId => Kv::ModelReleaseId(list),
        K::NameOfOrganisationFeaturedInTheImage => Kv::NameOfOrganisationFeaturedInTheImage(list),
        K::PersonShownInTheImage => Kv::PersonShownInTheImage(list),
        K::PropertyReleaseId => Kv::PropertyReleaseId(list),
        K::ModelAge => Kv::ModelAge(
            list.into_iter()
                .flat_map(|num_str| num_str.parse::<i64>())
                .collect(),
        ),

        // ignore non-vec types
        _ => return None,
    })
}

/// Parses a complex struct type that's in XMP format.
///
/// This is the shorthand parser - see also the [`self::parse_regular_struct_types`]
/// function for the other struct parser.
fn parse_shorthand_struct_types(key: K, element: &Element) -> Option<Kv> {
    // shorthand structs stick all their fields into their attributes.
    //
    // let's construct each compatible struct type from its attributes.
    //
    // note that only those with all string fields can be constructed this
    // way, so some "structish" variants won't appear here.
    //
    // also, since this is shorthand, we can only parse one single struct for
    // its values. that means we have a bunch of `Vec`s that will only contain
    // one element.
    //
    // FIXME: it seems technically possible to store some fields in attributes,
    //        and others as inner elements.
    //
    //        it'd be a massive pain in the ass to parse, but including it
    //        might improve compataibility.
    //
    //        please make an issue if you've found a sample with elements like
    //        this in the wild! we'll take a look.
    Some(match key {
        K::CreatorsContactInfo => Kv::CreatorsContactInfo(CreatorContactInfo {
            address: element.prefixed_attr(CreatorContactInfo::ADDRESS_XMP_ID),
            city: element.prefixed_attr(CreatorContactInfo::CITY_XMP_ID),
            country: element.prefixed_attr(CreatorContactInfo::COUNTRY_XMP_ID),
            emailwork: element.prefixed_attr(CreatorContactInfo::EMAILWORK_XMP_ID),
            phonework: element.prefixed_attr(CreatorContactInfo::PHONEWORK_XMP_ID),
            postal_code: element.prefixed_attr(CreatorContactInfo::POSTAL_CODE_XMP_ID),
            region: element.prefixed_attr(CreatorContactInfo::REGION_XMP_ID),
            weburlwork: element.prefixed_attr(CreatorContactInfo::WEBURLWORK_XMP_ID),
        }),

        K::CopyrightOwner => Kv::CopyrightOwner(vec![CopyrightOwner {
            copyright_owner_id: element.prefixed_attr(CopyrightOwner::COPYRIGHT_OWNER_ID_XMP_ID),
            copyright_owner_name: element
                .prefixed_attr(CopyrightOwner::COPYRIGHT_OWNER_NAME_XMP_ID),
        }]),

        K::CvTermAboutImage => Kv::CvTermAboutImage(vec![CvTerm {
            cv_id: element.prefixed_attr(CvTerm::CV_ID_XMP_ID),
            cv_term_id: element.prefixed_attr(CvTerm::CV_TERM_ID_XMP_ID),
            cv_term_name: element.prefixed_attr(CvTerm::CV_TERM_NAME_XMP_ID),
            cv_term_refined_about: element.prefixed_attr(CvTerm::CV_TERM_REFINED_ABOUT_XMP_ID),
        }]),

        K::EmbeddedEncodedRightsExpression => {
            Kv::EmbeddedEncodedRightsExpression(vec![EmbdEncRightsExpr {
                enc_rights_expr: element.prefixed_attr(EmbdEncRightsExpr::ENC_RIGHTS_EXPR_XMP_ID),
                rights_expr_enc_type: element
                    .prefixed_attr(EmbdEncRightsExpr::RIGHTS_EXPR_ENC_TYPE_XMP_ID),
                rights_expr_lang_id: element
                    .prefixed_attr(EmbdEncRightsExpr::RIGHTS_EXPR_LANG_ID_XMP_ID),
            }])
        }

        K::ImageRegistryEntry => Kv::ImageRegistryEntry(vec![RegistryEntry {
            asset_identifier: element.prefixed_attr(RegistryEntry::ASSET_IDENTIFIER_XMP_ID),
            registry_identifier: element.prefixed_attr(RegistryEntry::REGISTRY_IDENTIFIER_XMP_ID),
            role: element.prefixed_attr(RegistryEntry::ROLE_XMP_ID),
        }]),

        _ => return None,
    })
}

/// Parses a complex struct type that's in XMP format.
///
/// This is the regular parser - see also the [`self::parse_shorthand_struct_types`]
/// function for the other struct parser.
fn parse_regular_struct_types(key: K, element: &Element) -> Option<Kv> {
    // with the exception of fields' fields, regular structs tend to put their
    // data directly beneath them as inner elements.
    //
    // let's grab them!
    let mut inner: std::collections::HashMap<String, (&Element, Option<String>)> = element
        .children
        .iter()
        .flat_map(|node| node.as_element())
        .flat_map(|elem| Some((elem.prefixed_name(), (elem, elem.text_string()))))
        .collect();

    // quick helper to reduce boilerplate for taking the value directly outta
    // the hashmap.
    let mut get_mut = |key: &str| inner.get_mut(key).and_then(|i| i.1.take());

    Some(match key {
        K::CreatorsContactInfo => Kv::CreatorsContactInfo(CreatorContactInfo {
            address: get_mut(CreatorContactInfo::ADDRESS_XMP_ID),
            city: get_mut(CreatorContactInfo::CITY_XMP_ID),
            country: get_mut(CreatorContactInfo::COUNTRY_XMP_ID),
            emailwork: get_mut(CreatorContactInfo::EMAILWORK_XMP_ID),
            phonework: get_mut(CreatorContactInfo::PHONEWORK_XMP_ID),
            postal_code: get_mut(CreatorContactInfo::POSTAL_CODE_XMP_ID),
            region: get_mut(CreatorContactInfo::REGION_XMP_ID),
            weburlwork: get_mut(CreatorContactInfo::WEBURLWORK_XMP_ID),
        }),

        K::ArtworkOrObjectInTheImage => Kv::ArtworkOrObjectInTheImage({
            // so, our inner elements make up a list of types we parse into
            // this variant.
            //
            // let's start by grabbing the list's parent
            let parent: &Element = element.get_child("bag")?;

            // now, we can map each list entry into an `ArtworkOrObject` struct
            parent
                .children
                .iter()
                .flat_map(|li| -> Option<_> {
                    // this should be an `rdf:li` element, so let's make sure
                    // that's the case
                    let elem = li.as_element()?;
                    if elem.name != "rdf:li" {
                        return None;
                    }

                    // now, we can grab its inner elements to parse them into
                    // `ArtworkOrObject` structs
                    Some(ArtworkOrObject {
                        circa_date_created: elem
                            .prefixed_child(ArtworkOrObject::CIRCA_DATE_CREATED_XMP_ID)
                            .and_then(|e| e.text_string()),
                        content_description: elem
                            .self_or_li_text(ArtworkOrObject::CONTENT_DESCRIPTION_XMP_ID),
                        contribution_description: elem
                            .self_or_li_text(ArtworkOrObject::CONTRIBUTION_DESCRIPTION_XMP_ID),
                        copyright_notice: elem
                            .self_or_li_text(ArtworkOrObject::COPYRIGHT_NOTICE_XMP_ID),
                        creator_names: elem.all_li_texts(ArtworkOrObject::CREATOR_NAMES_XMP_ID),
                        creator_identifiers: elem
                            .all_li_texts(ArtworkOrObject::CREATOR_IDENTIFIERS_XMP_ID),
                        current_copyright_owner_identifier: elem.self_or_li_text(
                            ArtworkOrObject::CURRENT_COPYRIGHT_OWNER_IDENTIFIER_XMP_ID,
                        ),
                        current_copyright_owner_name: elem
                            .self_or_li_text(ArtworkOrObject::CURRENT_COPYRIGHT_OWNER_NAME_XMP_ID),
                        current_licensor_identifier: elem
                            .self_or_li_text(ArtworkOrObject::CURRENT_LICENSOR_IDENTIFIER_XMP_ID),
                        current_licensor_name: elem
                            .self_or_li_text(ArtworkOrObject::CURRENT_LICENSOR_NAME_XMP_ID),
                        date_created: elem.self_or_li_text(ArtworkOrObject::DATE_CREATED_XMP_ID),
                        physical_description: elem
                            .self_or_li_text(ArtworkOrObject::PHYSICAL_DESCRIPTION_XMP_ID),
                        source: elem.self_or_li_text(ArtworkOrObject::SOURCE_XMP_ID),
                        source_inventory_nr: elem
                            .self_or_li_text(ArtworkOrObject::SOURCE_INVENTORY_NR_XMP_ID),
                        source_inventory_url: elem
                            .self_or_li_text(ArtworkOrObject::SOURCE_INVENTORY_URL_XMP_ID),
                        style_period: elem.all_li_texts(ArtworkOrObject::STYLE_PERIOD_XMP_ID),
                        title: elem.self_or_li_text(ArtworkOrObject::TITLE_XMP_ID),
                    })
                })
                .collect()
        }),

        K::Contributor => Kv::Contributor({
            element
                .get_child("bag")?
                .children
                .iter()
                .flat_map(|li| -> Option<_> {
                    let elem = li.as_element()?;
                    if !elem.is_li_element() {
                        return None;
                    }

                    Some(EntityWRole {
                        identifiers: elem.all_li_texts(EntityWRole::IDENTIFIERS_XMP_ID),
                        name: elem.self_or_li_text(EntityWRole::NAME_XMP_ID),
                        role: elem.all_li_texts(EntityWRole::ROLE_XMP_ID),
                    })
                })
                .collect()
        }),

        K::CopyrightOwner => Kv::CopyrightOwner({
            element
                .get_child("bag")?
                .children
                .iter()
                .flat_map(|li| -> Option<_> {
                    let elem = li.as_element()?;
                    if !elem.is_li_element() {
                        return None;
                    }

                    Some(CopyrightOwner {
                        copyright_owner_id: elem
                            .self_or_li_text(CopyrightOwner::COPYRIGHT_OWNER_ID_XMP_ID),
                        copyright_owner_name: elem
                            .self_or_li_text(CopyrightOwner::COPYRIGHT_OWNER_NAME_XMP_ID),
                    })
                })
                .collect()
        }),

        K::CvTermAboutImage => Kv::CvTermAboutImage({
            element
                .get_child("bag")?
                .children
                .iter()
                .flat_map(|li| -> Option<_> {
                    let elem = li.as_element()?;
                    if !elem.is_li_element() {
                        return None;
                    }

                    Some(CvTerm {
                        cv_id: elem.self_or_li_text(CvTerm::CV_ID_XMP_ID),
                        cv_term_id: elem.self_or_li_text(CvTerm::CV_TERM_ID_XMP_ID),
                        cv_term_name: elem.self_or_li_text(CvTerm::CV_TERM_NAME_XMP_ID),
                        cv_term_refined_about: elem
                            .self_or_li_text(CvTerm::CV_TERM_REFINED_ABOUT_XMP_ID),
                    })
                })
                .collect()
        }),

        K::EmbeddedEncodedRightsExpression => Kv::EmbeddedEncodedRightsExpression({
            element
                .get_child("bag")?
                .children
                .iter()
                .flat_map(|li| -> Option<_> {
                    let elem = li.as_element()?;
                    if !elem.is_li_element() {
                        return None;
                    }

                    Some(EmbdEncRightsExpr {
                        enc_rights_expr: elem
                            .self_or_li_text(EmbdEncRightsExpr::ENC_RIGHTS_EXPR_XMP_ID),
                        rights_expr_enc_type: elem
                            .self_or_li_text(EmbdEncRightsExpr::RIGHTS_EXPR_ENC_TYPE_XMP_ID),
                        rights_expr_lang_id: elem
                            .self_or_li_text(EmbdEncRightsExpr::RIGHTS_EXPR_LANG_ID_XMP_ID),
                    })
                })
                .collect()
        }),

        K::Genre => Kv::Genre({
            element
                .get_child("bag")?
                .children
                .iter()
                .flat_map(|li| -> Option<_> {
                    let elem = li.as_element()?;
                    if !elem.is_li_element() {
                        return None;
                    }

                    Some(CvTerm {
                        cv_id: elem.self_or_li_text(CvTerm::CV_ID_XMP_ID),
                        cv_term_id: elem.self_or_li_text(CvTerm::CV_TERM_ID_XMP_ID),
                        cv_term_name: elem.self_or_li_text(CvTerm::CV_TERM_NAME_XMP_ID),
                        cv_term_refined_about: elem
                            .self_or_li_text(CvTerm::CV_TERM_REFINED_ABOUT_XMP_ID),
                    })
                })
                .collect()
        }),

        K::ImageCreator => Kv::ImageCreator({
            element
                .get_child("bag")?
                .children
                .iter()
                .flat_map(|li| -> Option<_> {
                    let elem = li.as_element()?;
                    if !elem.is_li_element() {
                        return None;
                    }

                    Some(ImageCreator {
                        image_creator_id: elem
                            .self_or_li_text(ImageCreator::IMAGE_CREATOR_ID_XMP_ID),

                        image_creator_name: elem
                            .self_or_li_text(ImageCreator::IMAGE_CREATOR_NAME_XMP_ID),
                    })
                })
                .collect()
        }),

        K::ImageRegion => Kv::ImageRegion({
            element
                .get_child("bag")?
                .children
                .iter()
                .flat_map(|li| -> Option<_> {
                    let elem = li.as_element()?;
                    if !elem.is_li_element() {
                        return None;
                    }

                    Some(ImageRegion {
                        region_boundary: elem
                            .prefixed_child(ImageRegion::REGION_BOUNDARY_XMP_ID)
                            .map(|e| RegionBoundary {
                                rb_shape: e.self_or_li_text(RegionBoundary::RB_SHAPE_XMP_ID),
                                rb_unit: e.self_or_li_text(RegionBoundary::RB_UNIT_XMP_ID),
                                rb_x: e
                                    .self_or_li_text(RegionBoundary::RB_X_XMP_ID)
                                    .and_then(|t| t.parse::<f64>().ok()),
                                rb_y: e
                                    .self_or_li_text(RegionBoundary::RB_Y_XMP_ID)
                                    .and_then(|t| t.parse::<f64>().ok()),
                                rb_w: e
                                    .self_or_li_text(RegionBoundary::RB_W_XMP_ID)
                                    .and_then(|t| t.parse::<f64>().ok()),
                                rb_h: e
                                    .self_or_li_text(RegionBoundary::RB_H_XMP_ID)
                                    .and_then(|t| t.parse::<f64>().ok()),
                                rb_rx: e
                                    .self_or_li_text(RegionBoundary::RB_RX_XMP_ID)
                                    .and_then(|t| t.parse::<f64>().ok()),

                                // this is a list of points
                                rb_vertices: e.prefixed_child("rdf:Seq").map(|list_elem| {
                                    list_elem
                                        .children
                                        .iter()
                                        .flat_map(|li| li.as_element())
                                        .filter(|li| li.is_li_element())
                                        .map(|li| {
                                            (
                                                li.prefixed_child(RegionBoundaryPoint::RB_X_XMP_ID),
                                                li.prefixed_child(RegionBoundaryPoint::RB_Y_XMP_ID),
                                            )
                                        })
                                        .flat_map(|(x, y)| {
                                            Some(RegionBoundaryPoint {
                                                rb_x: x?
                                                    .get_text()
                                                    .and_then(|t| t.parse::<f64>().ok()),
                                                rb_y: y?
                                                    .get_text()
                                                    .and_then(|t| t.parse::<f64>().ok()),
                                            })
                                        })
                                        .collect::<Vec<_>>()
                                }),
                            }),
                        r_id: elem.self_or_li_text(ImageRegion::R_ID_XMP_ID),
                        name: elem.self_or_li_text(ImageRegion::NAME_XMP_ID),
                        r_ctype: {
                            // grab first element to get its inner bag
                            let r_ctype_elem = elem.prefixed_child(ImageRegion::R_CTYPE_XMP_ID)?;

                            // grab `rdf:Bag`
                            let bag = r_ctype_elem.prefixed_child("rdf:Bag")?;

                            // map all the bag's children into `rdf:li`
                            Some(
                                bag.children
                                    .iter()
                                    .flat_map(|li| {
                                        // each `li` will contain one full `Entity` struct
                                        let li = li.as_element()?;

                                        if !li.is_li_element() {
                                            return None;
                                        }

                                        Some(Entity {
                                            identifiers: li
                                                .all_li_texts(Entity::IDENTIFIERS_XMP_ID),
                                            name: li.self_or_li_text(Entity::NAME_XMP_ID),
                                        })
                                    })
                                    .collect(),
                            )
                        },
                        r_role: {
                            // grab first element to get its inner bag
                            let r_role_elem = elem.prefixed_child(ImageRegion::R_ROLE_XMP_ID)?;

                            // grab `rdf:Bag`
                            let bag = r_role_elem.prefixed_child("rdf:Bag")?;

                            // map all the bag's children into `rdf:li`
                            Some(
                                bag.children
                                    .iter()
                                    .flat_map(|li| {
                                        // each `li` will contain one full `Entity` struct
                                        let li = li.as_element()?;

                                        if !li.is_li_element() {
                                            return None;
                                        }

                                        Some(Entity {
                                            identifiers: li
                                                .all_li_texts(Entity::IDENTIFIERS_XMP_ID),
                                            name: li.self_or_li_text(Entity::NAME_XMP_ID),
                                        })
                                    })
                                    .collect(),
                            )
                        },
                    })
                })
                .collect()
        }),

        K::ImageRegistryEntry => Kv::ImageRegistryEntry({
            // this expects a `Vec<RegistryEntry>` return type.
            //
            // so... let's get started on that. we'll:
            //
            // - grab `Iptc4xmpExt:RegistryId` from the XML
            // - take its `rdf:Bag` sub-element
            // - parse each `rdf:li` in the `rdf:Bag` as a `RegistryEntry`
            // - collect them into a Vec

            // start by grabbing registryid
            let reg_id_elem = element.prefixed_child(K::ImageRegistryEntry.xmp_id())?;

            // then its bag
            let bag = reg_id_elem.prefixed_child("rdf:Bag")?;

            // parse each `rdf:li` as an entry
            bag.children
                .iter()
                .flat_map(|bag_child_node| bag_child_node.as_element())
                .filter(|maybe_li| maybe_li.is_li_element())
                .map(|li| RegistryEntry {
                    asset_identifier: li
                        .prefixed_child(RegistryEntry::ASSET_IDENTIFIER_XMP_ID)
                        .and_then(|e| e.text_string()),
                    registry_identifier: li
                        .prefixed_child(RegistryEntry::REGISTRY_IDENTIFIER_XMP_ID)
                        .and_then(|e| e.text_string()),
                    role: li
                        .prefixed_child(RegistryEntry::ROLE_XMP_ID)
                        .and_then(|e| e.text_string()),
                })
                .collect()
        }),

        K::ImageSupplier => Kv::ImageSupplier({
            // this variant expects `Vec<ImageSupplier>`.
            //
            // - grab the main element from XML
            // - find a collection element at its top-level
            // - map each `rdf:li` sub-element into an `ImageSupplier` struct
            let img_supplier_elem = element.prefixed_child(K::ImageSupplier.xmp_id())?;
            let collection_elem = img_supplier_elem
                .children
                .iter()
                .flat_map(|n| n.as_element())
                .find(|e| e.is_collection_element())?;

            // map each sub-elem
            collection_elem
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .filter(|maybe_li| maybe_li.is_li_element())
                .map(|li| ImageSupplier {
                    image_supplier_id: li
                        .prefixed_child(ImageSupplier::IMAGE_SUPPLIER_ID_XMP_ID)
                        .and_then(|e| e.text_string()),
                    image_supplier_name: li
                        .prefixed_child(ImageSupplier::IMAGE_SUPPLIER_NAME_XMP_ID)
                        .and_then(|e| e.text_string()),
                })
                .collect()
        }),

        K::Licensor => Kv::Licensor({
            // expects a Vec<Licensor>
            let licensor_elem = element.prefixed_child(K::Licensor.xmp_id())?;
            let collection_elem = licensor_elem
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .find(|ce| ce.is_collection_element())?;

            collection_elem
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .filter(|maybe_li| maybe_li.is_li_element())
                .map(|li| Licensor {
                    licensor_id: li
                        .prefixed_child(Licensor::LICENSOR_ID_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_name: li
                        .prefixed_child(Licensor::LICENSOR_NAME_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_address: li
                        .prefixed_child(Licensor::LICENSOR_ADDRESS_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_address_detail: li
                        .prefixed_child(Licensor::LICENSOR_ADDRESS_DETAIL_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_city: li
                        .prefixed_child(Licensor::LICENSOR_CITY_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_state_province: li
                        .prefixed_child(Licensor::LICENSOR_STATE_PROVINCE_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_postal_code: li
                        .prefixed_child(Licensor::LICENSOR_POSTAL_CODE_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_country_name: li
                        .prefixed_child(Licensor::LICENSOR_COUNTRY_NAME_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_telephone_type1: li
                        .prefixed_child(Licensor::LICENSOR_TELEPHONE_TYPE1_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_telephone1: li
                        .prefixed_child(Licensor::LICENSOR_TELEPHONE1_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_telephone_type2: li
                        .prefixed_child(Licensor::LICENSOR_TELEPHONE_TYPE2_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_telephone2: li
                        .prefixed_child(Licensor::LICENSOR_TELEPHONE2_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_email: li
                        .prefixed_child(Licensor::LICENSOR_EMAIL_XMP_ID)
                        .and_then(|e| e.text_string()),
                    licensor_url: li
                        .prefixed_child(Licensor::LICENSOR_URL_XMP_ID)
                        .and_then(|e| e.text_string()),
                })
                .collect()
        }),

        K::LinkedEncodedRightsExpression => Kv::LinkedEncodedRightsExpression({
            element
                .prefixed_child(K::LinkedEncodedRightsExpression.xmp_id())?
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .find(|ce| ce.is_collection_element())?
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .filter(|maybe_li| maybe_li.is_li_element())
                .map(|li| LinkedEncRightsExpr {
                    linked_rights_expr: li
                        .prefixed_child(LinkedEncRightsExpr::LINKED_RIGHTS_EXPR_XMP_ID)
                        .and_then(|e| e.text_string()),
                    rights_expr_enc_type: li
                        .prefixed_child(LinkedEncRightsExpr::RIGHTS_EXPR_ENC_TYPE_XMP_ID)
                        .and_then(|e| e.text_string()),
                    rights_expr_lang_id: li
                        .prefixed_child(LinkedEncRightsExpr::RIGHTS_EXPR_LANG_ID_XMP_ID)
                        .and_then(|e| e.text_string()),
                })
                .collect()
        }),

        K::LocationCreated => Kv::LocationCreated({
            element
                .prefixed_child(K::LocationCreated.xmp_id())?
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .find(|ce| ce.is_collection_element())?
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .filter(|maybe_li| maybe_li.is_li_element())
                .map(location_from_li)
                .collect()
        }),

        K::LocationShownInTheImage => Kv::LocationShownInTheImage({
            // this is another `Vec<Location>` type.
            //
            // we'll:
            //
            // - grab the `Iptc4xmpExt:LocationShown` element
            // - find its `rdf:Bag` sub-element
            // - parse each `rdf:li` into a `Location` struct
            let loc_shown_elem = element.prefixed_child(K::LocationShownInTheImage.xmp_id())?;
            let collection_elem = loc_shown_elem
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .find(|ce| ce.is_collection_element())?;
            collection_elem
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .filter(|maybe_li| maybe_li.is_li_element())
                .map(location_from_li)
                .collect()
        }),

        K::PersonShownInTheImageWithDetails => Kv::PersonShownInTheImageWithDetails({
            // this makes a list of `Vec<PersonWDetails>` structs.
            //
            // we'll:
            //
            // - grab the `Iptc4xmpExt:PersonInImageWDetails` element
            // - find its `rdf:Bag` sub-element
            // - parse each `rdf:li` into a `PersonWDetails` struct
            let person_elem =
                element.prefixed_child(K::PersonShownInTheImageWithDetails.xmp_id())?;
            let collection_elem = person_elem
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .find(|ce| ce.is_collection_element())?;

            collection_elem
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .filter(|maybe_li| maybe_li.is_li_element())
                .flat_map(|li| {
                    Some(PersonWDetails {
                        identifiers: li.all_li_texts(PersonWDetails::IDENTIFIERS_XMP_ID),
                        name: li.self_or_li_text(PersonWDetails::NAME_XMP_ID),
                        characteristics: Some(
                            li.children
                                .iter()
                                .flat_map(|cn| cn.as_element())
                                .filter(|e| e.is_collection_element())
                                .map(|collection_elem: &Element| collection_elem.cvterm_from_list())
                                .collect(),
                        ),

                        description: li.self_or_li_text(PersonWDetails::DESCRIPTION_XMP_ID),
                    })
                })
                .collect()
        }),

        K::ProductShownInTheImage => Kv::ProductShownInTheImage({
            // create a `Vec<ProductWGtin>`.
            //
            // we'll:
            //
            // - grab the `Iptc4xmpExt:ProductInImage` element
            // - find its `rdf:Bag` sub-element
            // - parse each `rdf:li` into a `PersonWDetails` struct
            let product_elem = element.prefixed_child(K::ProductShownInTheImage.xmp_id())?;

            let collection_elem = product_elem
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .find(|ce| ce.is_collection_element())?;

            collection_elem
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .filter(|maybe_li| maybe_li.is_li_element())
                .flat_map(|li| {
                    Some(ProductWGtin {
                        identifiers: li.all_li_texts(ProductWGtin::IDENTIFIERS_XMP_ID),
                        name: li.self_or_li_text(ProductWGtin::NAME_XMP_ID),
                        gtin: li.self_or_li_text(ProductWGtin::GTIN_XMP_ID),
                        description: li.self_or_li_text(ProductWGtin::DESCRIPTION_XMP_ID),
                    })
                })
                .collect()
        }),

        _ => return None,
    })
}

/// A quick helper to get a Location from a collection of `rdf:li` elements.
///
/// This is used for `Iptc4xmpExt:LocationShownInTheImage`, for example.
///
/// Note that this function is infallible, as it assumes you've already checked
/// that this element is an `rdf:li` element, inside a collection element,
/// inside an element containing a list of `Location` structs.
fn location_from_li(li: &Element) -> Location {
    Location {
        city: li
            .prefixed_child(Location::CITY_XMP_ID)
            .and_then(|e| e.text_string()),
        country_code: li
            .prefixed_child(Location::COUNTRY_CODE_XMP_ID)
            .and_then(|e| e.text_string()),
        country_name: li
            .prefixed_child(Location::COUNTRY_NAME_XMP_ID)
            .and_then(|e| e.text_string()),
        gps_altitude: li
            .prefixed_child(Location::GPS_ALTITUDE_XMP_ID)
            .and_then(|e| e.text_string())
            .and_then(|s| s.parse::<f64>().ok()),
        gps_altitude_ref: li
            .prefixed_child(Location::GPS_ALTITUDE_REF_XMP_ID)
            .and_then(|e| e.text_string())
            .and_then(|s| s.parse::<f64>().ok()),
        gps_latitude: li
            .prefixed_child(Location::GPS_LATITUDE_XMP_ID)
            .and_then(|e| e.text_string())
            .and_then(|s| s.parse::<f64>().ok()),
        gps_longitude: li
            .prefixed_child(Location::GPS_LONGITUDE_XMP_ID)
            .and_then(|e| e.text_string())
            .and_then(|s| s.parse::<f64>().ok()),

        // this is a list, so we have to handle it carefully
        identifiers: li
            .prefixed_child(Location::IDENTIFIERS_XMP_ID)
            .and_then(|ident_elem| {
                // find the collection element containing the `rdf:li`
                let ident_collection = ident_elem
                    .children
                    .iter()
                    .flat_map(|ident_maybe_collection| ident_maybe_collection.as_element())
                    .find(|ident_maybe_collection| ident_maybe_collection.is_collection_element());

                // ...then, map each `rdf:li` into a string
                ident_collection.map(|collection_elem| {
                    collection_elem
                        .children
                        .iter()
                        .flat_map(|maybe_li| maybe_li.as_element())
                        .filter(|maybe_li| maybe_li.is_li_element())
                        .flat_map(|li| li.text_string())
                        .collect::<Vec<_>>()
                })
            }),
        name: li
            .prefixed_child(Location::NAME_XMP_ID)
            .and_then(|e| e.text_string()),
        province_state: li
            .prefixed_child(Location::PROVINCE_STATE_XMP_ID)
            .and_then(|e| e.text_string()),
        sublocation: li
            .prefixed_child(Location::SUBLOCATION_XMP_ID)
            .and_then(|e| e.text_string()),
        world_region: li
            .prefixed_child(Location::WORLD_REGION_XMP_ID)
            .and_then(|e| e.text_string()),
    }
}

/// This trait adds some helper methods on [`xmltree::Element`].
///
/// It does so to avoid overly verbose, reptitious usage of these patterns
/// across this file.
trait ElementExt {
    fn prefixed_child(&self, prefixed_name: &str) -> Option<&Element>;

    fn prefixed_name(&self) -> String;

    fn prefixed_attr(&self, key: &str) -> Option<String>;

    fn text_string(&self) -> Option<String>;

    fn get_list(&self) -> Vec<String>;

    fn get_first_alt_or_text(&self) -> Option<String>;

    fn cvterm_from_list(&self) -> CvTerm;

    fn all_li_texts(&self, key: &str) -> Option<Vec<String>>;

    fn self_or_li_text(&self, key: &str) -> Option<String>;

    /// Checks if `self` is a `rdf:li` element.
    fn is_li_element(&self) -> bool {
        self.prefixed_name().as_str() == "rdf:li"
    }

    /// Checks if `self` is any `rdf` collection element.
    ///
    /// These include `rdf:Alt`, `rdf:Bag`, and `rdf:Seq`.
    fn is_collection_element(&self) -> bool {
        ["rdf:Alt", "rdf:Bag", "rdf:Seq"].contains(&self.prefixed_name().as_str())
    }
}

impl ElementExt for Element {
    /// Searches for a child element with the given namespace prefix and name.
    fn prefixed_child(&self, prefixed_name: &str) -> Option<&Element> {
        // split the prefix and name
        let (prefix, name) = prefixed_name.split_once(':').or_else(|| {
            log::warn!("failed to split prefixed name: `{prefixed_name}`");

            #[cfg(debug_assertions)]
            panic!("given sub-element is not prefixed: `{prefixed_name}`");

            #[cfg(not(debug_assertions))]
            None
        })?;

        self.children
            .iter()
            .flat_map(|child| child.as_element())
            .find(|child| child.prefix == Some(prefix.to_string()) && child.name == name)
            .or_else(|| {
                #[cfg(debug_assertions)]
                panic!(
                    "failed to find prefixed child on element `{}`. prefix: `{}`",
                    self.name, prefixed_name
                );

                #[cfg(not(debug_assertions))]
                None
            })
    }

    /// Gets a child's name, prefixed with their namespace's prefix, if any.
    fn prefixed_name(&self) -> String {
        let mut res = match self.prefix.clone() {
            Some(mut prefix) => {
                prefix.push(':');
                prefix
            }
            None => String::new(),
        };

        res.push_str(&self.name);
        res
    }

    /// Gets an attribute from this element using the given prefixed name.
    ///
    /// Ex: `plus:CopyrightOwnerID`
    fn prefixed_attr(&self, key: &str) -> Option<String> {
        let (key_prefix, key_name) = key.split_once(':')?;

        self.attributes
            .iter()
            .flat_map(|(attr_prefix, attr_value)| {
                Some((
                    attr_prefix.prefix.clone()?,
                    attr_prefix.local_name.clone(),
                    attr_value,
                ))
            })
            .find(|(attr_prefix, attr_name, _attr_value)| {
                attr_prefix.as_str() == key_prefix && attr_name.as_str() == key_name
            })
            .map(|(_, _, attr_value)| attr_value.clone())
    }

    /// Grabs `self.text` as an `Option<String>`.
    fn text_string(&self) -> Option<String> {
        self.get_text().map(|cow| cow.to_string())
    }

    /// Helper function that grabs either:
    ///
    /// - An element's first sub-element on its `rdf:Alt` sub-element, or
    /// - The text of the element itself, if it has no `rdf:Alt` sub-element.
    ///
    /// This is helpful for entries like `Iptc4xmpExt:CvTermName`, which, despite
    /// being a single string, can sometimes be represented as a list of elements.
    fn get_first_alt_or_text(&self) -> Option<String> {
        // a closure that grab's the element's text.
        //
        // note that we'll try some fallible operations below. if any of them
        // fails, we'll return with this closure immediately.
        let grab_element_text = || self.text_string();

        // first, try to grab the `rdf:Alt` sub-element
        let Some(alt_elem) = self
            .children
            .iter()
            .flat_map(|cn| cn.as_element())
            .find(|ce| ce.name == "rdf:Alt")
        else {
            return grab_element_text();
        };

        // now, try to grab the first `rdf:li` sub-element
        let Some(first_li) = alt_elem
            .children
            .iter()
            .flat_map(|cn| cn.as_element())
            .find(|ce| ce.is_li_element())
        else {
            return grab_element_text();
        };

        // we found an `rdf:li` element! let's grab its text.
        //
        // if that fails, just return the original element's text
        first_li.text_string().or_else(grab_element_text)
    }

    /// A helper that parses an element that's a list. You should always provide
    /// this parser with the list parent - never the actual element.
    ///
    /// So, for example, you would provide the parser with `rdf:Alt` in the
    /// following sample XMP:
    ///
    /// ```xml
    /// <dc:title>
    ///     <rdf:Alt>
    ///         <rdf:li>Title 1</rdf:li>
    ///         <rdf:li>Title 2</rdf:li>
    ///         <rdf:li>Title 3</rdf:li>
    ///     </rdf:Alt>
    /// </dc:title>
    /// ```
    fn get_list(&self) -> Vec<String> {
        self.children
            .iter()
            .flat_map(|inner| inner.as_element())
            .filter(|inner| inner.is_collection_element())
            .flat_map(|inner| &inner.children)
            .flat_map(|list_node| list_node.as_element())
            .flat_map(|list_elem| list_elem.text_string())
            .collect()
    }

    /// Creates a `CvTerm` struct from a collection of `rdf:li` elements.
    ///
    /// This method assumes you're calling it on an `rdf:li` element that
    /// contains a `CvTerm` struct.
    fn cvterm_from_list(&self) -> CvTerm {
        CvTerm {
            cv_id: self
                .prefixed_child(CvTerm::CV_ID_XMP_ID)
                .and_then(|e| e.text_string()),
            cv_term_id: self
                .prefixed_child(CvTerm::CV_TERM_ID_XMP_ID)
                .and_then(|e| e.text_string()),
            cv_term_name: self
                .prefixed_child(CvTerm::CV_TERM_NAME_XMP_ID)
                .and_then(|e| e.get_first_alt_or_text()),
            cv_term_refined_about: self
                .prefixed_child(CvTerm::CV_TERM_REFINED_ABOUT_XMP_ID)
                .and_then(|e| e.text_string()),
        }
    }

    /// Helper to grab the text of every inner list element.
    ///
    /// Assumes you're calling this on an element that contains a container,
    /// such as an `rdf:Bag` or `rdf:Seq`, which contains `rdf:li` elements.
    fn all_li_texts(&self, key: &str) -> Option<Vec<String>> {
        // first, grab its container element
        let list_container = self
            .children
            .iter()
            .flat_map(|cn| cn.as_element())
            .find(|ce| ce.is_collection_element())?;

        // now, we can grab each `rdf:li` element's text
        Some(
            list_container
                .children
                .iter()
                .flat_map(|maybe_li_node| maybe_li_node.as_element())
                .filter(|maybe_li| maybe_li.is_li_element())
                .flat_map(|li| li.text_string())
                .collect(),
        )
    }

    // Helper to grab either:
    ///
    /// - an element's text, or...
    /// - the text of a potential inner single-item list
    ///     - but only if the parent element didn't have any text.
    ///
    /// This is useful for `Iptc4xmpExt:AOContentDescription`, for example.
    fn self_or_li_text(&self, key: &str) -> Option<String> {
        // if `maybe_text` is `None`, we can try grabbing the text from any
        // potential inner list...
        self.text_string().or_else(|| {
            // first, ensure that's a container element
            let list_container = self
                .children
                .iter()
                .flat_map(|cn| cn.as_element())
                .find(|ce| ce.is_collection_element())?;

            // now, we can parse the single list element
            let li = list_container
                .children
                .iter()
                .flat_map(|maybe_li_node| maybe_li_node.as_element())
                .find(|maybe_li| maybe_li.is_li_element())?;

            // finally, we can grab the text from the list element
            li.text_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use raves_metadata_types::iptc::IptcKeyValue;
    use xmltree::Element;

    use super::{ElementExt as _, parse_xmp_for_iptc};

    /// Tests the `ElementExt::get_list` and `ElementExt::prefixed_child`
    /// methods.
    #[test]
    fn get_list_and_prefixed_child() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .try_init();

        // parse out the document
        let element = Element::parse(r#"<dc:title xmlns:x="adobe:ns:meta/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Bag>
        <rdf:li>Title 1</rdf:li>
        <rdf:li>Title 2</rdf:li>
        <rdf:li>Title 3</rdf:li>
    </rdf:Bag>
</dc:title>"#.as_bytes())
        .expect("parse document");

        assert_eq!(
            element.get_list(),
            vec!["Title 1", "Title 2", "Title 3"],
            "list equality"
        );

        let dc_title = element
            .prefixed_child("rdf:Bag")
            .expect("can get prefixed child");

        assert_eq!(dc_title.prefix, Some("rdf".to_string()));
        assert_eq!(dc_title.name, "Bag");
    }

    /// Checks that we can grab a prefixed attribute.
    #[test]
    fn check_prefixed_attr() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .try_init();

        let element = Element::parse(r#"<dc:title xmlns:x="adobe:ns:meta/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:family="https://www.fox.com/family-guy/" family:guy="true"/>"#.as_bytes())
            .expect("parse document");

        let attr_val = element
            .prefixed_attr("family:guy")
            .expect("can get prefixed attribute");

        assert_eq!(attr_val, "true");
    }

    /// Checks that we can parse a very, very simple XMP sample with some IPTC
    /// internals.
    #[test]
    fn simple_xmp_iptc_embed() {
        _ = env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .try_init();

        let simple_xmp: &str = r#"<?xpacket begin="﻿" id="some_id"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns" xmlns:Iptc4xmpCore="http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/">
    <rdf:RDF>
        <rdf:Description Iptc4xmpCore:CountryCode="US" />
    </rdf:RDF>
</x:xmpmeta>"#;
        let parsed = parse_xmp_for_iptc(simple_xmp).expect("parsing should succeed");

        assert_eq!(
            parsed.pairs,
            vec![IptcKeyValue::CountryCodeLegacy("US".into())]
        );
    }
}

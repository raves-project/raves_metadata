use core::fmt::Write as _;
use std::{env, fs, path::Path};

use yaml_rust2::{Yaml, YamlLoader};

use crate::{
    ipmd_struct_creation::{IptcStruct, make_structs},
    ipmd_top_enum_creation::{EnumToGen, IptcEnum, make_enum},
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // load YAML spec
    let spec_path: &Path = Path::new("iptc-pmd-techreference_2024.1.yml");
    let yaml_src: String = fs::read_to_string(spec_path).expect("cannot read IPTC spec YAML file");
    let docs: Vec<Yaml> = YamlLoader::load_from_str(&yaml_src).expect("invalid YAML");
    let doc: &Yaml = &docs[0];

    // create a list of rust items we're gonna make
    let mut tys: RustTypes = Default::default();

    // make the enum from `ipmd_top`
    let raw_enum: &Yaml = &doc["ipmd_top"];
    tys.enum_ = make_enum(raw_enum);

    // and structs from `ipmd_struct`
    let raw_structs: &Yaml = &doc["ipmd_struct"];
    tys.structs = make_structs(raw_structs);

    // output to file
    tys.output_to_file();
}

#[derive(Default)]
pub struct RustTypes<'yaml> {
    // types
    enum_: IptcEnum<'yaml>,
    structs: Vec<IptcStruct<'yaml>>,
}

impl RustTypes<'_> {
    fn output_to_file(self) {
        let out_dir = env::var("OUT_DIR").unwrap();
        let dest = Path::new(&out_dir).join("iptc_keys.rs");

        fs::write(
            &dest,
            format!(
                r#"#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum IptcKeyValue {{
{}}}

{}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum IptcKey {{
{}}}

{}

{}

/// This module defines structures used within the enum variants of the IPTC
/// specification.
///
/// In particular, entries under `ipmd_struct` are mapped directly into Rust
/// types. These types are used in the `ipmd_top` variants.
pub mod structs {{
{}}}"#,
                // handle the enums
                self.enum_iptc_key_value(),
                self.generate_enum_impl(EnumToGen::IptcKeyValue),
                self.enum_iptc_key(),
                self.generate_enum_impl(EnumToGen::IptcKey),
                //
                // their `From` conversion
                self.generate_impl_iptc_key_kind_from_iptc_key(),
                //
                // rust structs in the `structs` module
                self.structs()
            ),
        )
        .expect("cannot write iptc_keys.rs");
    }

    /// Generates the primary `enum` of the crate, `IptcKeyValue`.
    ///
    /// This is the data-containing enum provided for pattern matching and
    /// usage within `raves_metadata`.
    fn enum_iptc_key_value(&self) -> String {
        const SPACING: &str = "    ";
        self.enum_.variants.iter().map(|v| (&v.ident, &v.ty)).fold(
            String::new(),
            |mut acc, (vari, ty)| {
                acc.push_str(SPACING);
                acc.push_str(vari);
                acc.push('(');
                acc.push_str(&ty.to_string());
                acc.push_str("),\n");
                acc
            },
        )
    }

    /// Generates the kind indicator enum, `IptcKey`.
    ///
    /// This can be created from an `IptcKeyValue` and serves as a compile-time
    /// reference for each variant's properties.
    fn enum_iptc_key(&self) -> String {
        const SPACING: &str = "    ";
        self.enum_
            .variants
            .iter()
            .map(|v| &v.ident)
            .fold(String::new(), |mut acc, vari| {
                acc.push_str(SPACING);
                acc.push_str(vari);
                acc.push_str(",\n");
                acc
            })
    }

    fn generate_impl_iptc_key_kind_from_iptc_key(&self) -> String {
        const START: &str =
            "impl ::core::convert::From<crate::iptc::IptcKeyValue> for crate::iptc::IptcKey {
    fn from(value: crate::iptc::IptcKeyValue) -> Self {
        match value {\n";

        // add the prefix + all the variants to it
        let mut s = self
            .enum_
            .variants
            .iter()
            .fold(String::from(START), |mut acc, v| {
                acc.push_str("            ");
                acc.push_str("crate::iptc::IptcKeyValue::");
                acc.push_str(&v.ident);
                acc.push_str(" { .. } => ");
                acc.push_str("crate::iptc::IptcKey::");
                acc.push_str(&v.ident);
                acc.push_str(",\n");

                acc
            });

        // wrap things up with the end brackets
        s.push_str("        }\n"); // ...end match
        s.push_str("    }\n"); // ...end fn
        s.push_str("}\n"); // ...end impl
        s
    }

    fn generate_enum_impl(&self, to_gen: EnumToGen) -> String {
        // make an iterator to shorten
        let it = move || self.enum_.variants.iter();

        // we can only make the `from_xmp_id` function if we're `IptcKey`.
        //
        // so, let's make it up here
        let from_xmp_id = match to_gen {
            EnumToGen::IptcKeyValue => "".into(),
            EnumToGen::IptcKey => format!(
                r#"
    pub fn from_xmp_id(s: impl AsRef<str>) -> Option<Self> {{
        let s_str: &str = s.as_ref();
        match s_str {{
{}        }}
    }}
"#,
                Self::match_variants(
                    it().map(|v| (&v.ident, Some(v.xmp))).collect::<Vec<_>>(),
                    to_gen
                ),
            ),
        };

        // alright, time to generate the actual impl block!
        format!(
            r#"impl {} {{
    pub const fn name(&self) -> &'static str {{
        match self {{
{}        }}
    }}

    pub const fn label(&self) -> &'static str {{
        match self {{
{}        }}
    }}

    pub const fn xmp_id(&self) -> &'static str {{
        match self {{
{}        }}
    }}

    pub const fn iim_id(&self) -> Option<&'static str> {{
        match self {{
{}        }}
    }}

    pub const fn iim_name(&self) -> Option<&'static str> {{
        match self {{
{}        }}
    }}

    pub const fn iim_max_bytes(&self) -> Option<u16> {{
        match self {{
{}        }}
    }}

    pub const fn exif_id(&self) -> Option<&'static str> {{
        match self {{
{}        }}
    }}

    /// Whether this variant has a primitive type as its data.
    pub const fn has_primitive_ty(&self) -> bool {{
        match self {{
{}        }}
    }}

    /// Whether this variant has a vector type (list of primitives) as its
    /// data.
    pub const fn has_vec_ty(&self) -> bool {{
        match self {{
{}        }}
    }}

    /// Whether this variant has a struct type as its data.
    pub const fn has_struct_ty(&self) -> bool {{
        match self {{
{}        }}
    }}
{}}}"#,
            to_gen,
            Self::match_arms(
                it().map(|v| (&v.ident, v.name)).collect::<Vec<_>>(),
                true,
                to_gen
            ),
            Self::match_arms(
                it().map(|v| (&v.ident, v.label)).collect::<Vec<_>>(),
                true,
                to_gen
            ),
            Self::match_arms(
                it().map(|v| (&v.ident, v.xmp)).collect::<Vec<_>>(),
                true,
                to_gen
            ),
            Self::optional_match_arms(
                it().map(|v| (&v.ident, v.iim_id)).collect::<Vec<_>>(),
                true,
                to_gen
            ),
            Self::optional_match_arms(
                it().map(|v| (&v.ident, v.iim_name)).collect::<Vec<_>>(),
                true,
                to_gen
            ),
            Self::optional_match_arms(
                it().map(|v| (&v.ident, v.iim_max)).collect::<Vec<_>>(),
                false,
                to_gen
            ),
            Self::optional_match_arms(
                it().map(|v| (&v.ident, v.exif)).collect::<Vec<_>>(),
                true,
                to_gen
            ),
            //
            // make data indicator types
            Self::match_arms(
                it().map(|v| (&v.ident, v.ty.is_primitive()))
                    .collect::<Vec<_>>(),
                false,
                to_gen
            ),
            Self::match_arms(
                it().map(|v| (&v.ident, v.ty.is_vec())).collect::<Vec<_>>(),
                false,
                to_gen
            ),
            Self::match_arms(
                it().map(|v| (&v.ident, v.ty.is_struct()))
                    .collect::<Vec<_>>(),
                false,
                to_gen
            ),
            from_xmp_id
        )
    }

    /// Generates the `structs` Rust module.
    ///
    /// It contains all the stuff from `ipmd_struct`.
    fn structs(&self) -> String {
        // we use a constant amount of spacing for each struct field
        const SPACING: &str = "    ";

        // for each struct, we'll make a definition
        self.structs
            .iter()
            .fold(String::new(), |mut acc, struct_entry| {
                // each struct gets its own Rust type
                acc.push_str(SPACING);
                acc.push_str("#[derive(Clone, Debug, PartialEq, PartialOrd)]\n");
                acc.push_str(SPACING);
                acc.push_str("pub struct ");
                acc.push_str(struct_entry.ident);
                acc.push_str(" {\n");

                // make a list of XMP ID constants.
                //
                // we'll add 'em in an `impl` block at the end
                let mut field_xmp_id_consts: Vec<String> =
                    Vec::with_capacity(struct_entry.fields.len());

                // add each field
                for field in &struct_entry.fields {
                    acc.push_str(SPACING);
                    acc.push_str(SPACING);
                    acc.push_str("pub ");

                    // add the field's name
                    acc.push_str(&field.ident);
                    acc.push_str(": ");

                    // and add the type...
                    acc.push_str(&field.ty.to_string());

                    // cap it off with a comma and a new line
                    acc.push_str(",\n");

                    // finally, add an entry for its constant
                    field_xmp_id_consts.push(format!(
                        "pub const {}_XMP_ID: &str = \"{}\";",
                        field.ident.to_ascii_uppercase(),
                        field.xmp_ident
                    ));
                }

                // cap it off with the brace and two newlines
                acc.push_str(SPACING);
                acc.push_str("}\n\n");

                // now, add its constants into a new `impl` block.
                //
                // make the impl block...
                acc.push_str(SPACING);
                acc.push_str("impl ");
                acc.push_str(struct_entry.ident);
                acc.push_str(" {\n");

                // add each `const`
                for const_xmp_id in field_xmp_id_consts {
                    acc.push_str(SPACING);
                    acc.push_str(SPACING);
                    acc.push_str(&const_xmp_id);
                    acc.push('\n');
                }

                // end the `impl` block
                acc.push_str(SPACING);
                acc.push_str("}\n\n");

                acc
            })
    }

    fn match_arms<VARI: core::fmt::Display, T: core::fmt::Display>(
        table: impl AsRef<Vec<(VARI, T)>>,
        use_quotes: bool,
        to_gen: EnumToGen,
    ) -> String {
        let prefix: String = format!("            {to_gen}::");
        const SUFFIX: &str = ",\n";

        let table = table.as_ref();

        table
            .iter()
            .fold(String::new(), |mut acc: String, (vari, t): &(_, T)| {
                acc.push_str(&prefix);

                // handle quotes
                let q = if use_quotes {
                    const { r#"""# }
                } else {
                    const { "" }
                };

                // print the type name
                write!(acc, "{vari}").unwrap();

                // if we're a struct-like variant, add the braces
                if to_gen == EnumToGen::IptcKeyValue {
                    acc.push_str(" { .. }");
                }
                write!(acc, " => ").unwrap();

                // add `T`
                write!(acc, "{q}{t}{q}").unwrap();

                acc.push_str(SUFFIX);
                acc
            })
    }

    fn optional_match_arms<VARI: core::fmt::Display, T: core::fmt::Display>(
        table: impl AsRef<Vec<(VARI, Option<T>)>>,
        use_quotes: bool,
        to_gen: EnumToGen,
    ) -> String {
        let prefix: String = format!("            {to_gen}::");
        const SUFFIX: &str = ",\n";

        let table = table.as_ref();

        table.iter().fold(
            String::new(),
            |mut acc: String, (vari, opt_t): &(_, Option<T>)| {
                acc.push_str(&prefix);

                // handle quotes
                let q = if use_quotes {
                    const { r#"""# }
                } else {
                    const { "" }
                };

                // print the type name
                write!(acc, "{vari}").unwrap();

                // if we're a struct-like variant, add the braces
                if to_gen == EnumToGen::IptcKeyValue {
                    acc.push_str(" { .. }");
                }
                write!(acc, " => ").unwrap();

                // either print None or Some(T)
                match opt_t {
                    Option::Some(t) => {
                        acc.push_str("Some(");
                        write!(acc, "{q}{t}{q}").unwrap();
                        acc.push(')');
                    }
                    Option::None => acc.push_str("None"),
                }

                acc.push_str(SUFFIX);
                acc
            },
        )
    }

    fn match_variants<VARI: core::fmt::Display, T: core::fmt::Display>(
        table: impl AsRef<Vec<(VARI, Option<T>)>>,
        to_gen: EnumToGen,
    ) -> String {
        const PREFIX: &str = r#"            "#;
        const SUFFIX: &str = ",\n";

        let table = table.as_ref();

        let mut s = table.iter().fold(
            String::new(),
            |mut acc: String, (vari, opt_t): &(_, Option<T>)| {
                // we can only provide a value => type mapping if there's a
                // value!
                //
                // so, for any None, we'll just skip em.
                let Some(t) = opt_t else {
                    return acc;
                };
                acc.push_str(PREFIX);

                // print the value
                write!(acc, "\"{t}\"").unwrap();

                // print the type name
                write!(acc, " => Some({to_gen}::").unwrap();
                write!(acc, "{vari})").unwrap();

                // add the suffix and wrap up
                acc.push_str(SUFFIX);
                acc
            },
        );

        // also, we have to add the final variant, which accounts for the case
        // where a user provides something not in the map.
        //
        // then, we can return it
        s.push_str(PREFIX);
        s.push_str("_ => None,\n");
        s
    }
}

/// This module provides `enum` creation functionality for IPTC types.
///
/// There are two main enums:
///
/// - `IptcKey`: only used to denote the names of `IptcKeyValue` variants
/// - `IptcKeyValue`: variants contain actual data
mod ipmd_top_enum_creation {
    use yaml_rust2::Yaml;

    use crate::{
        datatype::{Datatype, iptc_type_to_rust_type},
        string_case::kebab_case_to_pascal_case,
    };

    /// This is one of many enum variants in [`IptcEnum`].
    ///
    /// Each represents a possible type that an IPTC listing might provide.
    pub struct IptcEnumVariant<'yaml> {
        /// The variant's identifier.
        ///
        /// This is formatted in `PascalCase`.
        pub ident: String,

        /// The variant's data type. Converted from an IPTC `datatype` +
        /// `dataformat` into a suitable Rust type.
        pub ty: Datatype<'yaml>,

        // the rest of these fields are related to forming the methods +
        // associated functions for getting their static info (as defined in
        // the IPTC standard).
        //
        // note that these aren't accessors - instead, we make each field on
        // their internal data structs public
        pub name: &'yaml str,
        pub label: &'yaml str,
        pub xmp: &'yaml str,
        pub iim_id: Option<&'yaml str>,
        pub iim_name: Option<&'yaml str>,
        pub iim_max: Option<u16>,
        pub exif: Option<&'yaml str>,
    }

    /// An enum generated from the entries in `ipmd_top`.
    ///
    /// Note that we don't store the enum's identifier here, as this one struct
    /// actually represents two Rust enums that'll be created from its
    /// variants.
    ///
    /// But, for more info about that, see the module docs.
    #[derive(Default)]
    pub struct IptcEnum<'yaml> {
        /// A list of the enum's variants.
        pub variants: Vec<IptcEnumVariant<'yaml>>,
    }

    /// The enums we generate.
    #[derive(Copy, Clone, PartialEq)]
    pub enum EnumToGen {
        IptcKeyValue,
        IptcKey,
    }

    impl core::fmt::Display for EnumToGen {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                EnumToGen::IptcKeyValue => f.write_str("IptcKeyValue"),
                EnumToGen::IptcKey => f.write_str("IptcKey"),
            }
        }
    }

    /// This function creates the `IptcEnum` defining the rest of the crate.
    ///
    /// `ipmd_top` is just the `ipmd_top` YAML set. Don't do any other parsing
    /// past the initial hash.
    pub fn make_enum(ipmd_top: &Yaml) -> IptcEnum {
        // unwrap the enum info from its parent
        let raw_enums = ipmd_top
            .as_hash()
            .expect("grabbing list of `ipmd_top` types");

        // make a blank enum; we'll return it later
        let mut iptc_enum: IptcEnum = IptcEnum {
            variants: Vec::with_capacity(raw_enums.len()),
        };

        // map each YAML key into a Rust enum variant.
        //
        // we'll also add constants for it in our match maps
        for (variant_key, variant_value) in raw_enums {
            // grab the variant's description (all its subelements)
            let variant_desc = variant_value.as_hash().expect("variants are parents");

            // grab its identifier...
            let ident: String = kebab_case_to_pascal_case(
                raw_enums[variant_key]
                    .as_hash()
                    .expect("key can become hash")
                    .get(&Yaml::String("specidx".into()))
                    .unwrap_or_else(|| panic!("`{variant_desc:?}` doesn't have `specidx`!!!!"))
                    .as_str()
                    .expect("specidx field is required to create identifier"),
            );

            // ...and its type
            let ty: Datatype = iptc_type_to_rust_type(
                from_yaml(variant_desc, "datatype"),
                optional_from_yaml(variant_desc, "propoccurrence")
                    .and_then(|value| match value {
                        "single" => Some(false),
                        "multi" => Some(true),
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("should know propoccurrence for variant: {ident}")),
                true,
                optional_from_yaml(variant_desc, "dataformat"),
            );

            // here's the data collected on each variant. we'll use it to make
            // an info struct...
            let info = IptcEnumVariant {
                // for generating the enum type
                ident,
                ty,

                // for creating enum methods
                name: from_yaml(variant_desc, "name"),
                label: from_yaml(variant_desc, "label"),
                xmp: from_yaml(variant_desc, "XMPid"),
                iim_id: option_from_yaml(variant_desc, "IIMid"),
                iim_name: optional_from_yaml(variant_desc, "IIMname"),
                iim_max: optional_number_from_yaml(variant_desc, "IIMmaxbytes"),
                exif: optional_from_yaml(variant_desc, "EXIFid"),
            };

            // add it to the enum's collection
            iptc_enum.variants.push(info);
        }

        iptc_enum
    }

    fn from_yaml<'yaml>(hash: &'yaml yaml_rust2::yaml::Hash, field: &str) -> &'yaml str {
        if let Some(yaml) = hash.get(&yaml_rust2::Yaml::from_str(field)) {
            if let Some(yaml_str) = yaml.as_str() {
                return yaml_str;
            }
        }

        panic!("required YAML key not found. field: {field}");
    }

    /// note: this one has a real Option.
    fn option_from_yaml<'yaml>(
        hash: &'yaml yaml_rust2::yaml::Hash,
        field: &str,
    ) -> Option<&'yaml str> {
        if let Some(yaml) = hash.get(&yaml_rust2::Yaml::from_str(field)) {
            if let Some(yaml_str) = yaml.as_str() {
                // any empty strings must become None; otherwise, we get dupes in
                // our match maps
                if !yaml_str.trim().is_empty() {
                    return Some(yaml_str);
                }
            }
        }

        None
    }

    fn optional_from_yaml<'yaml>(
        hash: &'yaml yaml_rust2::yaml::Hash,
        field: &str,
    ) -> Option<&'yaml str> {
        if let Some(yaml) = hash.get(&yaml_rust2::Yaml::from_str(field)) {
            if let Some(yaml_str) = yaml.as_str() {
                // any empty strings must become None; otherwise, we get dupes in
                // our match maps
                if !yaml_str.trim().is_empty() {
                    return Some(yaml_str);
                }
            }
        }

        None
    }

    fn optional_number_from_yaml(hash: &yaml_rust2::yaml::Hash, field: &str) -> Option<u16> {
        if let Some(yaml) = hash.get(&yaml_rust2::Yaml::from_str(field)) {
            if let Some(yaml_i64) = yaml.as_i64() {
                return Some(yaml_i64 as u16);
            }
        }

        None
    }
}

/// The following implements `ipmd_struct` => Rust `struct` creation.
///
/// Why? Well, in essence, IPTC is defined by two categories:
///
/// 1. `ipmd_top`
/// 2. `ipmd_struct`
///
/// The structures in `ipmd_top` can reference structures defined in
/// `ipmd_struct`, so we need to include the `struct` stuff as Rust structs
/// directly.
///
/// This module does just that...
mod ipmd_struct_creation {
    use std::collections::HashMap;

    use yaml_rust2::Yaml;

    use crate::{
        datatype::{Datatype, iptc_type_to_rust_type},
        string_case::camel_case_to_snake_case,
    };

    /// A single field inside a larger [`IptcStruct`].
    pub struct IptcStructField<'yaml> {
        /// A struct field's name.
        ///
        /// Ex: in `foo: Bar`, the `ident` is `foo`!
        pub ident: String,

        /// The field's type, like `Option<u32>`.
        ///
        /// Ex: in `foo: Bar`, the `ty` is `Bar`.
        pub ty: Datatype<'yaml>,

        /// This isn't really related to the creation of field itself, but each
        /// in the standard is also assigned an XMP identifier.
        ///
        /// That's pretty helpful for parsing, so we'll remember it here.
        pub xmp_ident: &'yaml str,
    }

    /// A full Rust struct generated from an entry in `ipmd_struct`.
    pub struct IptcStruct<'yaml> {
        /// The struct's name.
        ///
        /// Ex: it's `Foo` inside of: `struct Foo {}`.
        pub ident: &'yaml str,

        /// A list of fields contained inside the struct.
        pub fields: Vec<IptcStructField<'yaml>>,
    }

    /// Creates a list of Rust structs from a given set of `ipmd_structs`.
    ///
    /// These map directly from the types defined in YAML.
    pub fn make_structs(ipmd_structs: &Yaml) -> Vec<IptcStruct> {
        // we're given an object, but we want the inner list of stuff.
        //
        // this is exposed as a type: `&LinkedHashMap<Yaml, Yaml>`, but
        // `yaml_rust2` forgot to re-export that type, so it's more private
        // than the method lol
        let raw_structs = ipmd_structs
            .as_hash()
            .expect("grabbing inner `ipmd_struct` list of types");

        // here's the finished list we'll return.
        //
        // it's pre-allocated with more than enough space
        let mut structs: Vec<IptcStruct> = Vec::with_capacity(raw_structs.len());

        // we'll iterate over each entry and create a struct for it.
        for (struct_ident_raw, members_raw) in raw_structs {
            // grab the yaml list members
            let members = members_raw
                .as_hash()
                .expect("list of members can become list");

            // use the struct's raw YAML identifier as its Rust name.
            //
            // why? well... no `specidx` exists on the actual structs. :(
            let struct_ident: &str = struct_ident_raw.as_str().expect("grab struct name");

            // scan its members into fields
            let struct_fields: Vec<IptcStructField> = members
                .into_iter()
                .flat_map(|(member_key, member_value)| -> Option<_> {
                    let field_ident: &str = member_key.as_str()?;
                    let field_desc: HashMap<&Yaml, &Yaml> =
                        member_value.as_hash()?.into_iter().collect();

                    make_field(field_ident, field_desc)
                })
                .collect();

            structs.push(IptcStruct {
                ident: struct_ident,
                fields: struct_fields,
            });
        }

        structs
    }

    /// Creates a struct field from a given YAML identifier and its listed
    /// description.
    fn make_field<'yaml>(
        camel_case_ident: &'yaml str,
        desc: HashMap<&'yaml Yaml, &'yaml Yaml>,
    ) -> Option<IptcStructField<'yaml>> {
        // ignore the any type included in the standard (`anypmdproperty`).
        //
        // it'd require special handling, and currently, there is no listed
        // value that makes use of it.
        if camel_case_ident.starts_with("$") {
            return None;
        }

        // we need to parse the field's description for these things:
        //
        // 1. `datatype`: maps to `Ty` in `iden: Ty`
        // 2. `dataformat`: says to use another struct as the field's type
        // 3. `propoccurrence`: whether to use `Vec<Ty>` instead
        // 4. `isrequired`:  whether to use `Option<T>` instead
        // 5. `XMPid`: the unique XMP identifier for this field
        //
        // then, we can generate a field for it!
        let mut maybe_datatype: Option<&str> = None;
        let mut dataformat: Option<&str> = None; // this is optional - we don't unwrap it
        let mut maybe_prop_occurrence_multi: Option<bool> = None;
        let mut maybe_required: Option<bool> = None;
        let mut maybe_xmp_ident: Option<&str> = None;
        for (raw_prop_key, raw_prop_value) in desc {
            let prop_key: &str = raw_prop_key.as_str()?;
            let prop_value: &str = raw_prop_value.as_str()?;

            match prop_key {
                // for the data ty + format, we'll use those directly to make
                // the field.
                "datatype" => maybe_datatype = Some(prop_value),
                "dataformat" => dataformat = Some(prop_value),

                // the property occurance mentions how many times something can
                // be specified. we use it to make a `Vec<Thing>` instead of
                // just `Thing`.
                //
                // so, we map that into a boolean
                "propoccurrence" => match prop_value {
                    "single" => maybe_prop_occurrence_multi = Some(false),
                    "multi" => maybe_prop_occurrence_multi = Some(true),
                    _ => (),
                },

                // this specifies whether we should use `Option<T>` or `T`.
                //
                // note that, as of writing, the standard requests `false` for
                // all provided types lol
                "isrequired" => match prop_value {
                    "0" => maybe_required = Some(false),
                    "1" => maybe_required = Some(true),
                    _ => (),
                },

                // we can use this directly
                "XMPid" => maybe_xmp_ident = Some(prop_value),

                // ignore any other option; we don't care about them
                _ => (),
            }
        }

        // great - now we need to unwrap all those...
        let (datatype, multi, required, xmp_ident) = (
            maybe_datatype.expect("get property dataty"),
            maybe_prop_occurrence_multi.expect("get property occurance"),
            maybe_required.expect("get property: `isrequired`"),
            maybe_xmp_ident.expect("get xmp ident"),
        );

        // map the ident into a Rusty name
        let ident: String = camel_case_to_snake_case(camel_case_ident);

        // map the IPTC type into a Rust type
        let ty: Datatype = iptc_type_to_rust_type(datatype, multi, required, dataformat);

        // ...and construct the field!
        Some(IptcStructField {
            ident,
            ty,
            xmp_ident,
        })
    }
}

/// This small module converts from various string formats into others.
///
/// It allows the crate to have "real-looking" Rust types.
pub mod string_case {
    /// Converts a `kebab-cased` string slice into a `PascalCased` string.
    ///
    /// Useful for `specidx` format. For example: `specidx: '#an-example-case'`
    pub fn kebab_case_to_pascal_case(kebab: &str) -> String {
        kebab
            .chars()
            .fold(
                (false, String::with_capacity(kebab.len())),
                |(capitalize_char, mut acc), cha| {
                    // we're going to ignore some characters:
                    //
                    // `#`: the first character. we don't use it
                    // `-`: just a separator between words
                    if cha == '#' || cha == '-' {
                        return (true, acc);
                    }

                    // any of these characters simply aren't helpful for us, so
                    // we'll get rid of them
                    if cha == '\'' {
                        return (capitalize_char, acc);
                    }

                    // otherwise, map the character accordingly
                    if capitalize_char {
                        acc.push(cha.to_ascii_uppercase());
                    } else {
                        acc.push(cha);
                    }

                    // false - we're not given a reason to capitalize the next character
                    // in the iterator here.
                    (false, acc)
                },
            )
            .1 // we want to return the string
    }

    /// Converts a `kebab-cased` string slice into a `snake_case` string.
    ///
    /// Useful for `specidx` format. For example: `specidx: '#an-example-case'`
    pub fn kebab_case_to_snake_case(kebab: &str) -> String {
        kebab
            .chars()
            .fold(String::with_capacity(kebab.len()), |mut acc, cha| {
                // skip any `#` characters
                if cha == '#' {
                    return acc;
                }

                // replace `-` with `_`
                if cha == '-' {
                    acc.push('_');
                    return acc;
                }

                // all other characters are already lowercase.
                //
                // so, add it and return
                acc.push(cha);
                acc
            })
    }

    /// Converts `camelCase` identifiers into `snake_case`.
    pub fn camel_case_to_snake_case(original: &str) -> String {
        original
            .chars()
            .fold(String::with_capacity(original.len()), |mut acc, mut cha| {
                // uppercase letters become lowercase, prefixed with an underscore.
                //
                // for example, "aT" => "a_t"
                if cha.is_uppercase() {
                    acc.push('_');
                    acc.push({
                        cha.make_ascii_lowercase();
                        cha
                    });
                } else {
                    acc.push(cha);
                }

                acc
            })
    }
}

/// Related to parsing `datatype` into something usable.
pub mod datatype {
    use std::fmt::Write as _;

    /// A Rust type stemming from an IPTC `datatype`, inclusive of vecs.
    pub struct Datatype<'yaml> {
        /// Whether this is a `Vec<T>` or just a `T`.
        pub vec: bool,

        /// Whether `T` is required - if false, this is `Option<T>`
        pub required: bool,

        /// The `T` mentioned above.
        pub kind: DatatypeKind<'yaml>,
    }

    /// Rust representation of an IPTC `datatype`.
    pub enum DatatypeKind<'yaml> {
        /// A primitive string - no parsing needed!
        String,

        /// A primitive number specified to be an integer.
        ///
        /// We'll use `i64` to ensure we've got enough bits.
        Integer,

        /// A primitive number not specified to be an integer.
        ///
        /// These are usually floats, so we'll assume as such.
        MaybeFloat,

        /// A type from the `ipmd_struct` list.
        CrateType(&'yaml str),
    }

    impl core::fmt::Display for Datatype<'_> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            if !self.required {
                f.write_str("::core::option::Option<")?;
            }

            if self.vec {
                f.write_str("::alloc::vec::Vec<")?;
            }

            match self.kind {
                DatatypeKind::String => f.write_str("::alloc::string::String")?,
                DatatypeKind::Integer => f.write_str("i64")?,
                DatatypeKind::MaybeFloat => f.write_str("f64")?,
                DatatypeKind::CrateType(iptc_struct_ident) => {
                    f.write_str("crate::iptc::structs::")?;
                    f.write_str(iptc_struct_ident)?;
                }
            }

            if self.vec {
                f.write_char('>')?;
            }

            if !self.required {
                f.write_char('>')?;
            }

            Ok(())
        }
    }

    impl Datatype<'_> {
        /// Checks if we're a primitive.
        pub fn is_primitive(&self) -> bool {
            !self.vec
                && match self.kind {
                    DatatypeKind::Integer | DatatypeKind::MaybeFloat | DatatypeKind::String => true,
                    DatatypeKind::CrateType(..) => false,
                }
        }

        /// Checks if we're a vec.
        pub fn is_vec(&self) -> bool {
            self.vec
        }

        /// Checks if we're a struct.
        pub fn is_struct(&self) -> bool {
            match self.kind {
                DatatypeKind::CrateType(..) => true,
                DatatypeKind::Integer | DatatypeKind::MaybeFloat | DatatypeKind::String => false,
            }
        }
    }

    /// Converts an IPTC type into a Rust type.
    pub fn iptc_type_to_rust_type<'yaml>(
        iptc_type: &'yaml str,
        multi: bool,
        required: bool,
        dataformat: Option<&'yaml str>,
    ) -> Datatype<'yaml> {
        let mut kind: Option<DatatypeKind> = None;

        // there are some fields which refer to builtin types or other
        // generated types.
        //
        // we need to handle those first.
        if iptc_type == "struct" {
            if let Some(referential_ty) = dataformat {
                match referential_ty {
                    "AltLang" => kind = Some(DatatypeKind::String),
                    "uri" | "url" => kind = Some(DatatypeKind::String),
                    "date-time" => kind = Some(DatatypeKind::String),

                    // other structs not listed here are defined in `ipmd_struct`
                    // itself!
                    //
                    // so, we'll use their Rust types directly
                    other => kind = Some(DatatypeKind::CrateType(other)),
                }
            }
        } else {
            // handle normal types by mapping them to Rust primitives
            match iptc_type {
                "string" => kind = Some(DatatypeKind::String),
                "number" => {
                    kind = Some(if dataformat == Some("integer") {
                        DatatypeKind::Integer
                    } else {
                        DatatypeKind::MaybeFloat
                    })
                }

                // we shouldn't build for other types. let's ensure it doesn't
                // compile by panicking...
                other => panic!("other type: {other}"),
            };
        }

        // finally, wrap all that up into a type
        Datatype {
            vec: multi,
            required,
            kind: kind.expect("should have type"),
        }
    }
}

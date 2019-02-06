// Copyright 2019 Ian Castleden
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use serde_derive_internals::{ast, attr::EnumTag};

use super::{derive_field, ident_from_str, type_to_ts, QuoteT};

struct TagInfo<'a> {
    tag: &'a str,
    content: Option<&'a str>,
}
pub(crate) fn derive_enum<'a>(variants: &[ast::Variant<'a>], container: &ast::Container) -> QuoteT {
    // let n = variants.len() - 1;
    let taginfo = match container.attrs.tag() {
        EnumTag::Internal { tag, .. } => TagInfo { tag, content: None },
        EnumTag::Adjacent { tag, content, .. } => TagInfo {
            tag,
            content: Some(&content),
        },
        _ => TagInfo {
            tag: "kind",
            content: None,
        },
    };
    let content = variants.iter().map(|variant| {
        let variant_name = variant.attrs.name().serialize_name();
        match variant.style {
            ast::Style::Struct => derive_struct_variant(&taginfo, &variant_name, &variant.fields, container),
            ast::Style::Newtype => {
                derive_newtype_variant(&taginfo, &variant_name, &variant.fields[0])
            }
            ast::Style::Tuple => derive_tuple_variant(&taginfo, &variant_name, &variant.fields),
            ast::Style::Unit => derive_unit_variant(&taginfo, &variant_name),
        }
    });
    // OK generate A | B | C etc
    quote! { #(#content)|* }
}

fn derive_unit_variant(taginfo: &TagInfo, variant_name: &str) -> QuoteT {
    let tag = ident_from_str(taginfo.tag);
    quote! {
        { #tag: #variant_name }
    }
}

fn derive_newtype_variant<'a>(
    taginfo: &TagInfo,
    variant_name: &str,
    field: &ast::Field<'a>,
) -> QuoteT {
    let ty = type_to_ts(&field.ty);
    let tag = ident_from_str(taginfo.tag);
    let content = if let Some(content) = taginfo.content {
        ident_from_str(&content)
    } else {
        ident_from_str("fields")
    };

    quote! {
        { #tag: #variant_name, #content: #ty }
    }
}

fn derive_struct_variant<'a>(
    taginfo: &TagInfo,
    variant_name: &str,
    fields: &[ast::Field<'a>],
    container: &ast::Container,
) -> QuoteT {
    use std::collections::HashSet;
    let contents = fields.iter().map(|field| derive_field(field));


    let tag = ident_from_str(taginfo.tag);
    if let Some(content) = taginfo.content {
        let content = ident_from_str(&content);
        quote! {
            { #tag: #variant_name, #content: { #(#contents),* } }
        }
    } else {
        let fnames = fields.iter().map(|field| field.attrs.name().serialize_name()).collect::<HashSet<_>>();
        if fnames.contains(taginfo.tag) {
            panic!("tag \"{}\" clashes with field in Enum variant \"{};:{}\"", taginfo.tag, container.ident, variant_name );
        }
        quote! {
            { #tag: #variant_name, #(#contents),* }
        }
    }
}

fn derive_tuple_variant<'a>(
    taginfo: &TagInfo,
    variant_name: &str,
    fields: &[ast::Field<'a>],
) -> QuoteT {
    let contents = fields.iter().map(|field| type_to_ts(&field.ty));
    // .collect::<Vec<_>>();

    let tag = ident_from_str(taginfo.tag);
    let content = if let Some(content) = taginfo.content {
        ident_from_str(&content)
    } else {
        ident_from_str("fields")
    };

    quote! {
     { #tag: #variant_name, #content : [ #(#contents),* ] }
    }
}

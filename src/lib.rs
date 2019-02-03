extern crate proc_macro;

#[macro_use]
extern crate quote;

// extern crate serde;
extern crate proc_macro2;
extern crate regex;
extern crate serde_derive_internals;
extern crate syn;
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "bytes")]
extern crate serde_bytes;

use proc_macro2::Span;
use quote::TokenStreamExt;
use regex::{Captures, Regex};
use serde_derive_internals::{ast, Ctxt, Derive};
use syn::DeriveInput;

mod derive_enum;
mod derive_struct;

type QuoteT = proc_macro2::TokenStream;

lazy_static! {
    static ref RE: Regex =
        Regex::new(r"(?P<nl>\n+)|(?P<brack>\s*\[\s+\])|(?P<brace>\{\s+\})").unwrap();
}
// TODO: where does the newline come from? why the double spaces?

trait Has {
    fn has(&self, s: &'static str) -> bool;
}

impl Has for Captures<'_> {
    #[inline]
    fn has(&self, s: &'static str) -> bool {
        self.name(s).is_some()
    }
}

fn debug_patch(s: &str) -> String {
    RE.replace_all(s, |c: &Captures| {
        if c.has("brace") {
            "{ }"
        } else if c.has("brack") {
            " [ ]"
        } else {
            assert!(c.has("nl"));
            " "
        }
    })
    .into()
}

fn patch(s: &str) -> String {
    RE.replace_all(s, |c: &Captures| {
        if c.has("brace") {
            "{}"
        } else if c.has("brack") {
            "[]"
        } else {
            assert!(c.has("nl"));
            "\n"
        }
    })
    .into()
}

/* // TODO: where does the newline come from? why the double spaces?
fn debug_patch2(s: &str) -> String {
    s.replace("\n", " ")
        .replace("[  ]", "[ ]")
        .replace("{  }", "{ }")
}

fn patch2(s: &str) -> String {
    s.replace(" [  ]", "[]").replace(" {  }", "{}")
} */

fn parse(input: proc_macro::TokenStream) -> (syn::Ident, Vec<QuoteT>, QuoteT) {
    // eprintln!(".........[input] {}", input);
    let input: DeriveInput = syn::parse(input).unwrap();

    let cx = Ctxt::new();
    let container = ast::Container::from_ast(&cx, &input, Derive::Deserialize);

    let typescript = match container.data {
        ast::Data::Enum(variants) => derive_enum::derive_enum(&variants, &container.attrs),
        ast::Data::Struct(style, fields) => {
            derive_struct::derive_struct(style, &fields, &container.attrs)
        }
    };

    let lifetimes = generic_lifetimes(container.generics);

    // consumes context
    cx.check().unwrap();
    (container.ident, lifetimes, typescript)
}

#[proc_macro_derive(TypescriptDefinition)]
pub fn derive_typescript_definition(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (ident, _lifetimes, typescript) = parse(input);

    let typescript_string = typescript.to_string();
    let export_string = format!("export type {} = {};", ident, patch(&typescript_string));

    let export_ident = ident_from_str(&format!("TS_EXPORT_{}", ident.to_string().to_uppercase()));

    // eprintln!(
    //     "....[typescript] export type {}={};",
    //     ident, typescript_string
    // );

    let mut expanded = quote! {

        #[wasm_bindgen(typescript_custom_section)]
        const #export_ident : &'static str = #export_string;
    };

    if cfg!(any(debug_assertions, feature = "test-export")) {
        let ts = debug_patch(&typescript_string); // why the newlines?
        let typescript_ident = ident_from_str(&format!("{}___typescript_definition", ident));

        expanded.append_all(quote! {
            fn #typescript_ident ( ) -> &'static str {
                #ts
            }
        })
    }

    expanded.into()
}

#[proc_macro_derive(TypeScriptify)]
pub fn derive_type_script_ify(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (ident, lifetimes, typescript) = parse(input);

    let typescript_string = patch(&typescript.to_string());
    let export_string = format!("export type {} = {};", ident, typescript_string);

    let ret = if lifetimes.len() == 0 {
        quote! {

            impl TypeScriptifyTrait for #ident {
                fn type_script_ify() ->  &'static str {
                    #export_string
                }
            }
        }
    } else {
        // can't use 'a need '_
        let lt = lifetimes.iter().map(|_q| quote!('_)); // .collect::<Vec<_>>();

        quote! {

            impl TypeScriptifyTrait for #ident<#(#lt),*> {
                fn type_script_ify() ->  &'static str {
                    #export_string
                }
            }
        }
    };

    ret.into()
}

fn generic_lifetimes(g: &syn::Generics) -> Vec<QuoteT> {
    // get all the generic lifetimes
    // we ignore type parameters because we can't
    // reasonably serialize generic structs! But
    // std::borrow::Cow; requires a lifetime parameter ... see tests/typescript.rs
    use syn::{GenericParam, LifetimeDef};
    g.params
        .iter()
        .filter_map(|p| match p {
            GenericParam::Lifetime(LifetimeDef { lifetime, .. }) => Some(lifetime),
            _ => None,
        })
        .map(|lt| quote!(#lt))
        .collect::<Vec<_>>()
}

fn ident_from_str(s: &str) -> proc_macro2::Ident {
    syn::Ident::new(s, Span::call_site())
}

// fn collapse_list_comma(body: &[QuoteT]) -> QuoteT {
//     let n = body.len() - 1;

//     body.into_iter()
//         .enumerate()
//         .fold(quote! {}, |mut agg, (i, tokens)| {
//             agg.append_all(if i < n {
//                 quote!(#tokens,)
//             } else {
//                 tokens.clone()
//             });
//             agg
//         })
// }
// fn collapse_list_bar(body: &[QuoteT]) -> QuoteT {
//     // let n = body.len() - 1;

//     // body.into_iter()
//     //     .enumerate()
//     //     .fold(quote! {}, |mut agg, (i, tokens)| {
//     //         agg.append_all(if i < n {
//     //             quote!(#tokens | )
//     //         } else {
//     //             tokens.clone()
//     //         });
//     //         agg
//     //     })
//     quote! { #(#body)|* }
// }

// fn collapse_list_bracket(body: &[QuoteT]) -> QuoteT {
//     // let tokens = collapse_list_comma(body);

//     quote! { [ #(#body),* ] }
// }

// fn collapse_list_brace(body: &[QuoteT]) -> QuoteT {
//     // let tokens = collapse_list_comma(body);
//     quote! { { #(#body),* } }
// }

fn type_array(elem: &syn::Type) -> QuoteT {
    let tp = type_to_ts(elem);
    quote! { #tp[] }
}

struct TSType {
    ident: syn::Ident,
    args: Vec<QuoteT>,
}
fn last_path(path: &syn::Path) -> Option<TSType> {
    match path.segments.last().map(|p| p.into_value()) {
        Some(t) => {
            let ident = t.ident.clone();
            let args = match &t.arguments {
                syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    args,
                    ..
                }) => args,
                //syn::PathArguments::Parenthesized(syn::ParenthesizedGenericArguments { inputs, output, ..}) => {
                //    return Some(TSType { ident:ident, args:vec![]})
                //}
                _ => {
                    return Some(TSType {
                        ident: ident,
                        args: vec![],
                    })
                }
            };

            let args = args
                .iter()
                .filter_map(|p| match p {
                    syn::GenericArgument::Type(t) => Some(t),
                    _ => None,
                })
                .map(|p| type_to_ts(p))
                .collect::<Vec<_>>();

            Some(TSType {
                ident: ident,
                args: args,
            })
        }
        None => None,
    }
}
fn generic_to_ts(ts: TSType) -> QuoteT {
    match ts.ident.to_string().as_ref() {
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128"
        | "isize" | "f64" | "f32" => quote! { number },
        "String" | "str" => quote! { string },
        "bool" => quote! { boolean },
        "Vec" => {
            let args = &ts.args;
            // let a = collapse_list_bar(&ts.args);
            quote! { #(#args)|*[] }
        }
        "Cow" if ts.args.len() > 0 => ts.args[0].clone(),
        "HashMap" if ts.args.len() >= 2 => {
            let k = &ts.args[0];
            let v = &ts.args[1];
            quote!(Map<#k,#v>)
        }
        "HashSet" if ts.args.len() >= 1 => {
            let k = &ts.args[0];
            quote!(Set<#k>)
        }
        "Option" if ts.args.len() >= 1 => {
            let k = &ts.args[0];
            quote!(#k | null)
        }
        "Result" if ts.args.len() >= 2 => {
            let k = &ts.args[0];
            let v = &ts.args[1];
            quote!(#k | #v)
        }
        _ => {
            let ident = ts.ident;
            if ts.args.len() > 0 {
                let args = ts.args;
                //let args = collapse_list_comma(&ts.args);
                quote! { #ident<#(#args),*> }
            } else {
                quote! {#ident}
            }
        }
    }
}

fn type_to_ts(ty: &syn::Type) -> QuoteT {
    use syn::Type::*;
    use syn::{
        TypeArray, TypeGroup, TypeImplTrait, TypeParamBound, TypeParen, TypePath, TypePtr,
        TypeReference, TypeSlice, TypeTraitObject, TypeTuple,
    };
    match ty {
        Slice(TypeSlice { elem, .. }) => type_array(elem),
        Array(TypeArray { elem, .. }) => type_array(elem),
        Ptr(TypePtr { elem, .. }) => type_array(elem),
        Reference(TypeReference { elem, .. }) => type_to_ts(elem),
        BareFn(..) => quote! { any },
        Never(..) => quote! { never },
        Tuple(TypeTuple { elems, .. }) => {
            let qelems = elems.iter().map(|t| type_to_ts(t)); // .collect::<Vec<_>>();
            quote!([ #(#qelems),* ])
            // collapse_list_bracket(&qelems)
        }
        Path(TypePath { path, .. }) => match last_path(&path) {
            Some(ts) => generic_to_ts(ts),
            _ => quote! { any },
        },
        TraitObject(TypeTraitObject { bounds, .. }) | ImplTrait(TypeImplTrait { bounds, .. }) => {
            let qelems = bounds
                .iter()
                .filter_map(|t| match t {
                    TypeParamBound::Trait(t) => last_path(&t.path),
                    _ => None, // skip lifetime etc.
                })
                .map(|t| {
                    let ident = t.ident;
                    quote!(#ident)
                });
            // .collect::<Vec<_>>();
            //collapse_list_bar(&qelems)
            quote!(#(#qelems)|*)
        }
        Paren(TypeParen { elem, .. }) => {
            let tp = type_to_ts(elem);
            quote! { ( #tp ) }
        }
        Group(TypeGroup { elem, .. }) => type_to_ts(elem),
        Infer(..) => quote! { any },
        Macro(..) => quote! { any },
        Verbatim(..) => quote! { any },
    }
}

fn derive_field<'a>(field: &ast::Field<'a>) -> QuoteT {
    let field_name = field.attrs.name().serialize_name();
    let ty = type_to_ts(&field.ty);
    quote! {
        #field_name: #ty
    }
}

/* fn derive_element<'a>(field: &ast::Field<'a>) -> QuoteT {
    type_to_ts(&field.ty)
} */

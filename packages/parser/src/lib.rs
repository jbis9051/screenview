extern crate proc_macro;

use proc_macro2::{TokenStream, Span};
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Error, GenericArgument, LitInt, PathArguments, Result,
    Type, Ident
};
use proc_macro_crate::{crate_name, FoundCrate};

mod gen;
mod parse;

#[proc_macro_derive(MessageComponent, attributes(parse))]
pub fn derive_message_component(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let crate_common = common();

    match &input.data {
        Data::Struct(data_struct) => {
            let fields = match parse::parse_fields(data_struct) {
                Ok(fields) => fields,
                Err(e) => return e.into_compile_error().into(),
            };
            gen::gen_struct_impl(&crate_common, &input, &fields).into()
        }
        _ => Error::new_spanned(&input, "ADT not supported")
            .into_compile_error()
            .into(),
    }
}

#[proc_macro_attribute]
pub fn message_id(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let id: LitInt = match syn::parse(attr) {
        Ok(int) => int,
        Err(e) => {
            return Error::new(e.span(), "Expected integer literal")
                .to_compile_error()
                .into()
        }
    };

    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let crate_common = common();

    (quote! {
        #input

        impl #impl_generics #crate_common::messages::MessageID for #name #ty_generics #where_clause {
            const ID: u8 = #id;
        }
    })
    .into()
}

pub(crate) fn matches_ident(ty: &Type, ident: &str) -> bool {
    match ty {
        Type::Path(path) => {
            path.qself.is_none()
                && path.path.leading_colon.is_none()
                && !path.path.segments.is_empty()
                && path.path.segments.last().unwrap().ident == ident
        }
        _ => false,
    }
}

pub(crate) fn extract_type_from_container(ty: &Type) -> Result<Type> {
    match ty {
        Type::Slice(slice) => Ok(slice.elem.as_ref().clone()),
        Type::Path(path) => {
            let type_params = &path.path.segments.last().unwrap().arguments;

            let generic_arg = match type_params {
                PathArguments::AngleBracketed(params) => params.args.first().unwrap(),
                tokens => return Err(Error::new_spanned(tokens, "Expected type parameter")),
            };

            match generic_arg {
                GenericArgument::Type(ty) => Ok(ty.clone()),
                arg => Err(Error::new_spanned(arg, "Expected type parameter")),
            }
        }
        ty => Err(Error::new_spanned(ty, "Expected path type")),
    }
}

pub(crate) fn common() -> TokenStream {
    match crate_name("common") {
        Ok(FoundCrate::Itself) => quote! { crate },
        Ok(FoundCrate::Name(name)) => {
            let name = Ident::new(&name, Span::call_site());
            quote! { ::#name }
        }
        Err(e) => Error::new(Span::call_site(), format!("{}", e)).to_compile_error(),
    }
}

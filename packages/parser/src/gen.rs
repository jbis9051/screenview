use crate::parse::{ArrayLength, ArrayType, Condition, Field, TypeInfo};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, LitInt};

pub fn gen_struct_impl(crate_common: &TokenStream, input: &DeriveInput, fields: &[Field]) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let read = gen_struct_deserialize_impl(crate_common, fields);
    let write = gen_struct_serialize_impl(crate_common, fields);

    quote! {
        impl #impl_generics #crate_common::messages::MessageComponent for #name #ty_generics #where_clause {
            fn read(__cursor: &mut ::std::io::Cursor<&[u8]>) -> Result<Self, #crate_common::messages::Error> {
                #read
            }

            fn write(&self, __cursor: &mut ::std::io::Cursor<::std::vec::Vec<u8>>) -> Result<(), #crate_common::messages::Error> {
                #write
            }
        }
    }
}

fn gen_struct_serialize_impl(crate_common: &TokenStream, fields: &[Field]) -> TokenStream {
    let serialize_fields = fields.iter().map(|field| gen_serialize_struct_field(crate_common, field));

    quote! {
        #( #serialize_fields )*
        Ok(())
    }
}

fn gen_struct_deserialize_impl(crate_common: &TokenStream, fields: &[Field]) -> TokenStream {
    let deserialize_fields = fields
        .iter()
        .map(|field| gen_deserialize_struct_field(crate_common, field));
    let field_names = fields.iter().map(|field| &field.ident);

    quote! {
        #( #deserialize_fields )*
        Ok(Self {
            #( #field_names ),*
        })
    }
}

fn gen_serialize_struct_field(crate_common: &TokenStream, field: &Field) -> TokenStream {
    let name = &field.ident;

    match &field.type_info {
        TypeInfo::OptionArray {
            condition,
            length,
            inner,
            ..
        } => {
            let field_ref = quote! { __value };
            let condition = condition.gen_write_condition(&quote! { self.#name });
            let length = length.gen_write_length(&field_ref);
            let write = inner.gen_write_impl(crate_common, &field_ref);

            quote! {
                #condition
                if let ::core::option::Option::Some(#field_ref) = &self.#name {
                    #length
                    #write
                }
            }
        }
        TypeInfo::Option { condition, .. } => {
            let field_ref = quote! { __value };
            let condition = condition.gen_write_condition(&quote! { self.#name });

            quote! {
                #condition
                if let ::core::option::Option::Some(#field_ref) = &self.#name {
                    #crate_common::messages::MessageComponent::write(#field_ref, __cursor)?;
                }
            }
        }
        TypeInfo::Array { length, inner, .. } => {
            let field_ref = quote! { self.#name };
            let length = length.gen_write_length(&field_ref);
            let write = inner.gen_write_impl(crate_common, &field_ref);

            quote! {
                #length
                #write
            }
        }
        TypeInfo::Regular(_) => {
            quote! {
                #crate_common::messages::MessageComponent::write(&self.#name, __cursor)?;
            }
        }
    }
}

fn gen_deserialize_struct_field(crate_common: &TokenStream, field: &Field) -> TokenStream {
    let name = &field.ident;

    match &field.type_info {
        TypeInfo::OptionArray {
            condition,
            length,
            outer,
            inner,
        } => {
            let present = condition.gen_read_condition(crate_common);
            let len = length.gen_read_length();
            let read = inner.gen_read_impl(crate_common);

            quote! {
                let #name: #outer = if #present {
                    #len
                    #read
                    Some(__dest)
                } else {
                    None
                };
            }
        }
        TypeInfo::Option {
            condition, outer, ..
        } => {
            let present = condition.gen_read_condition(crate_common);

            quote! {
                let #name: #outer = if #present {
                    Some(#crate_common::messages::MessageComponent::read(__cursor)?)
                } else {
                    None
                };
            }
        }
        TypeInfo::Array {
            length,
            outer,
            inner,
        } => {
            let len = length.gen_read_length();
            let read = inner.gen_read_impl(crate_common);

            quote! {
                let #name: #outer = {
                    #len
                    #read
                    __dest
                };
            }
        }
        TypeInfo::Regular(ty) => {
            quote! {
                let #name: #ty = #crate_common::messages::MessageComponent::read(__cursor)?;
            }
        }
    }
}

impl ArrayLength {
    fn gen_write_length(&self, field_ref: &TokenStream) -> Option<TokenStream> {
        match self {
            Self::Expr(_) | Self::Greedy(_) | Self::Fixed(_) => None,
            Self::Prefixed(_, bytes) => Some(gen_int_write_fn(&quote! { #field_ref.len() }, bytes)),
        }
    }

    fn gen_read_length(&self) -> TokenStream {
        match self {
            Self::Expr(expr) => quote! { let __len = #expr; },
            Self::Greedy(_) => {
                quote! { let __len = __cursor.get_ref().len().saturating_sub(usize::try_from(__cursor.position())?); }
            }
            Self::Fixed(len) => quote! { let __len = #len; },
            Self::Prefixed(_, bytes) => {
                let read_len = gen_int_read_fn(bytes);
                quote! { let __len = usize::try_from(#read_len)?; }
            }
        }
    }
}

impl Condition {
    fn gen_write_condition(&self, field_ref: &TokenStream) -> Option<TokenStream> {
        match self {
            Self::Expr(_) => None,
            Self::Prefixed(_) => Some(quote! {
                ::byteorder::WriteBytesExt::write_u8(__cursor, u8::from(#field_ref.is_some()))?;
            }),
        }
    }

    fn gen_read_condition(&self, crate_common: &TokenStream) -> TokenStream {
        match self {
            Self::Expr(expr) => quote! { #expr },
            Self::Prefixed(_) => quote! { <bool as #crate_common::messages::MessageComponent>::read(__cursor)? },
        }
    }
}

impl ArrayType {
    fn gen_write_impl(&self, crate_common: &TokenStream, field_ref: &TokenStream) -> TokenStream {
        match self {
            Self::Vec(_) => {
                quote! {
                   for __ele in #field_ref.iter() {
                        #crate_common::messages::MessageComponent::write(__ele, __cursor)?;
                    }
                }
            }
            Self::String => {
                quote! {
                    ::std::io::Write::write_all(__cursor, #field_ref.as_bytes())?;
                }
            }
        }
    }

    fn gen_read_impl(&self, crate_common: &TokenStream) -> TokenStream {
        match self {
            Self::Vec(_) => {
                quote! {
                    let mut __dest = Vec::with_capacity(__len);
                    for _ in 0..__len {
                        __dest.push(#crate_common::messages::MessageComponent::read(__cursor)?);
                    }
                }
            }
            Self::String => {
                quote! {
                    let mut __dest = ::std::vec![0u8; __len];
                    ::std::io::Read::read_exact(__cursor, &mut __dest)?;
                    let __dest = ::std::string::String::from_utf8(__dest)?;
                }
            }
        }
    }
}

fn gen_int_write_fn(expr: &TokenStream, bytes: &LitInt) -> TokenStream {
    match bytes.base10_parse::<u8>() {
        Ok(1) => quote! {
            ::byteorder::WriteBytesExt::write_u8(__cursor, u8::try_from(#expr)?)?;
        },
        Ok(2) => quote! {
            ::byteorder::WriteBytesExt::write_u16::<::byteorder::LittleEndian>(__cursor, u16::try_from(#expr)?)?;
        },
        Ok(3) => quote! {
            ::byteorder::WriteBytesExt::write_u24::<::byteorder::LittleEndian>(__cursor, u32::try_from(#expr)?)?;
        },
        Ok(4) => quote! {
            ::byteorder::WriteBytesExt::write_u32::<::byteorder::LittleEndian>(__cursor, u32::try_from(#expr)?)?;
        },
        Ok(8) => quote! {
            ::byteorder::WriteBytesExt::write_u64::<::byteorder::LittleEndian>(__cursor, u64::try_from(#expr)?)?;
        },
        Ok(_) => Error::new_spanned(bytes, "invalid integer byte size").to_compile_error(),
        Err(e) => Error::new_spanned(bytes, &format!("failed to parse integer byte size: {}", e))
            .to_compile_error(),
    }
}

fn gen_int_read_fn(bytes: &LitInt) -> TokenStream {
    match bytes.base10_parse::<u8>() {
        Ok(1) => quote! {
            ::byteorder::ReadBytesExt::read_u8(__cursor)?
        },
        Ok(2) => quote! {
            ::byteorder::ReadBytesExt::read_u16::<::byteorder::LittleEndian>(__cursor)?
        },
        Ok(3) => quote! {
            ::byteorder::ReadBytesExt::read_u24::<::byteorder::LittleEndian>(__cursor)?
        },
        Ok(4) => quote! {
            ::byteorder::ReadBytesExt::read_u32::<::byteorder::LittleEndian>(__cursor)?
        },
        Ok(8) => quote! {
            ::byteorder::ReadBytesExt::read_u64::<::byteorder::LittleEndian>(__cursor)?
        },
        Ok(_) => Error::new_spanned(bytes, "invalid integer byte size").to_compile_error(),
        Err(e) => Error::new_spanned(bytes, &format!("failed to parse integer byte size: {}", e))
            .to_compile_error(),
    }
}

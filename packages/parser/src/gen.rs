use proc_macro2::TokenStream;
use syn::{DeriveInput, LitInt, Error};
use crate::parse::{Field, ArrayLength, Condition, TypeInfo};
use quote::quote;

pub fn gen_struct_impl(input: &DeriveInput, fields: &[Field]) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let read = gen_struct_deserialize_impl(fields);
    let write = gen_struct_serialize_impl(fields);

    quote! {
        impl #impl_generics ::crate::MessageComponent for #name #ty_generics #where_clause {
            fn read(__cursor: &mut ::std::io::Cursor<&[u8]>) -> Result<Self, Error> {
                #read
            }

            fn write(&self, __cursor: &mut ::std::io::Cursor<::std::vec::Vec<u8>>) -> ::std::io::Result<()> {
                #write
            }
        }
    }
}

fn gen_struct_serialize_impl(fields: &[Field]) -> TokenStream {
    let serialize_fields = fields.iter()
        .map(|field| gen_serialize_struct_field(field));

    quote! {
        #( #serialize_fields )*
        Ok(())
    }
}

fn gen_struct_deserialize_impl(fields: &[Field]) -> TokenStream {
    let deserialize_fields = fields.iter().map(|field| gen_deserialize_struct_field(field));
    let field_names = fields.iter().map(|field| &field.ident);

    quote! {
        Ok(Self {
            #( #deserialize_fields )*
            #( #field_names ),*
        })
    }
}

fn gen_serialize_struct_field(field: &Field) -> TokenStream {
    let name = &field.ident;

    match &field.type_info {
        TypeInfo::OptionVector { condition, length, .. } => {
            let field_ref = quote! { __value };
            let condition = condition.gen_write_condition(&quote! { self.#name });
            let length = length.gen_write_length(&field_ref);

            quote! {
                #condition
                if let ::core::option::Option::Some(#field_ref) = &self.#name {
                    #length
                    for __ele in #field_ref {
                        ::crate::MessageComponent::write(__cursor, __ele)?;
                    }
                }
            }
        },
        TypeInfo::Option { condition, .. } => {
            let field_ref = quote! { __value };
            let condition = condition.gen_write_condition(&quote! { &self.#name });

            quote! {
                #condition
                if let ::core::option::Option::Some(#field_ref) = &self.#name {
                    ::crate::MessageComponent::write(#field_ref, __cursor)?;
                }
            }
        },
        TypeInfo::Vector { length, .. } => {
            let field_ref = quote! { self.#name };
            let length = length.gen_write_length(&field_ref);

            quote! {
                #length
                for __ele in &#field_ref {
                    ::crate::MessageComponent::write(__ele, __cursor)?;
                }
            }
        },
        TypeInfo::Regular(_) => {
            quote! {
                ::crate::MessageComponent::write(&self.#name, __cursor)?;
            }
        }
    }
}

fn gen_deserialize_struct_field(field: &Field) -> TokenStream {
    let name = &field.ident;

    match &field.type_info {
        TypeInfo::OptionVector { condition, length, outer, .. } => {
            let present = condition.gen_read_condition();
            let len = length.gen_read_length();

            quote! {
                let #name: #outer = if #present {
                    #len
                    let mut __dest = Vec::with_capacity(__len);
                    for _ in 0..__len {
                        __dest.push(::common::MessageComponent::read(__cursor)?);
                    }
                    Some(__dest)
                } else {
                    None
                };
            }
        },
        TypeInfo::Option { condition, outer, .. } => {
            let present = condition.gen_read_condition();

            quote! {
                let #name: #outer = if #present {
                    Some(::common::MessageComponent::read(__cursor)?)
                } else {
                    None
                };
            }
        },
        TypeInfo::Vector { length, outer, .. } => {
            let len = length.gen_read_length();

            quote! {
                let #name: #outer = {
                    #len
                    let mut __dest = Vec::with_capacity(__len);
                    for _ in 0..__len {
                        __dest.push(::common::MessageComponent::read(__cursor)?);
                    }
                    __dest
                };
            }
        },
        TypeInfo::Regular(ty) => {
            quote! {
                let #name: #ty = ::crate::MessageComponent::read(__cursor)?;
            }
        }
    }
}

impl ArrayLength {
    fn gen_write_length(&self, field_ref: &TokenStream) -> Option<TokenStream> {
        match self {
            Self::Expr(_) | Self::Greedy(_) | Self::Fixed(_) => None,
            Self::Prefixed(_, bytes) => Some(gen_int_write_fn(&quote! { #field_ref.len() }, bytes))
        }
    }

    fn gen_read_length(&self) -> TokenStream {
        match self {
            Self::Expr(expr) => quote! { let __len = #expr; },
            Self::Greedy(_) => quote! { let __len = __cursor.get_ref().len().saturating_sub(usize::from(__cursor.position())); },
            Self::Fixed(len) => quote! { let __len = #len; },
            Self::Prefixed(_, bytes) => {
                let read_len = gen_int_read_fn(bytes);
                quote! { let __len = #read_len; }
            }
        }
    }
}

impl Condition {
    fn gen_write_condition(&self, field_ref: &TokenStream) -> Option<TokenStream> {
        match self {
            Self::Expr(_) => None,
            Self::Prefixed(_) => Some(quote! {
                ::byteorder::WriteBytesExt::write_u8(__cursor, #field_ref.is_some() as u8)?;
            })
        }
    }

    fn gen_read_condition(&self) -> TokenStream {
        match self {
            Self::Expr(expr) => quote! { #expr },
            Self::Prefixed(_) => quote! { ::byteorder::ReadBytesExt::read_u8(__cursor)? == 1 }
        }
    }
}

fn gen_int_write_fn(expr: &TokenStream, bytes: &LitInt) -> TokenStream {
    match bytes.base10_parse::<u8>() {
        Ok(1) => quote! {
            ::byteorder::WriteBytesExt::write_u8(__cursor, (#expr) as u8)?;
        },
        Ok(2) => quote! {
            ::byteorder::WriteBytesExt::write_u16::<::byteorder::LittleEndian>(__cursor, (#expr) as u16)?;
        },
        Ok(3) => quote! {
            ::byteorder::WriteBytesExt::write_u24::<::byteorder::LittleEndian>(__cursor, (#expr) as u32)?;
        },
        Ok(4) => quote! {
            ::byteorder::WriteBytesExt::write_u32::<::byteorder::LittleEndian>(__cursor, (#expr) as u32)?;
        },
        Ok(8) => quote! {
            ::byteorder::WriteBytesExt::write_u64::<::byteorder::LittleEndian>(__cursor, (#expr) as u64)?;
        },
        Ok(_) => Error::new_spanned(bytes, "invalid integer byte size").to_compile_error(),
        Err(e) => Error::new_spanned(bytes, &format!("failed to parse integer byte size: {}", e)).to_compile_error()
    }
}

fn gen_int_read_fn(bytes: &LitInt) -> TokenStream {
    match bytes.base10_parse::<u8>() {
        Ok(1) => quote! {
            ::byteorder::ReadBytesExt::read_u8(__cursor)?;
        },
        Ok(2) => quote! {
            ::byteorder::ReadBytesExt::read_u16::<::byteorder::LittleEndian>(__cursor)?;
        },
        Ok(3) => quote! {
            ::byteorder::ReadBytesExt::read_u24::<::byteorder::LittleEndian>(__cursor)?;
        },
        Ok(4) => quote! {
            ::byteorder::ReadBytesExt::read_u32::<::byteorder::LittleEndian>(__cursor)?;
        },
        Ok(8) => quote! {
            ::byteorder::ReadBytesExt::read_u64::<::byteorder::LittleEndian>(__cursor)?;
        },
        Ok(_) => Error::new_spanned(bytes, "invalid integer byte size").to_compile_error(),
        Err(e) => Error::new_spanned(bytes, &format!("failed to parse integer byte size: {}", e)).to_compile_error()
    }
}

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse2, DataStruct, Error, Expr, Fields, Ident, LitInt, LitStr, Result, Token, Type,
};

use crate::{extract_type_from_container, matches_ident};

pub fn parse_fields(data: &DataStruct) -> Result<Vec<Field>> {
    let fields = match &data.fields {
        Fields::Named(named) => named,
        _ => return Err(Error::new_spanned(&data.fields, "Fields must be named")),
    };

    let mut dest = Vec::with_capacity(fields.named.len());
    for field in &fields.named {
        let attr = field.attrs.iter().find(|&attr| attr.path.is_ident("parse"));
        let params = attr
            .map(|attr| parse2::<ParseParams>(attr.tokens.clone()))
            .transpose()?
            .unwrap_or_default();

        let outer_ty = field.ty.clone();
        let mut is_option = false;
        let mut is_array = false;
        let mut inner_ty = outer_ty.clone();
        let mut inner_ty_info = None;

        if matches_ident(&inner_ty, "Option") {
            is_option = true;
            inner_ty = extract_type_from_container(&inner_ty)?;
            if params.condition.is_none() {
                return Err(Error::new_spanned(attr, "missing condition parameter"));
            }
        } else if params.condition.is_some() {
            return Err(Error::new_spanned(
                attr,
                "conditions can only be applied to option types",
            ));
        }

        if matches_ident(&inner_ty, "Vec") {
            is_array = true;
            inner_ty = extract_type_from_container(&inner_ty)?;
            inner_ty_info = Some(ArrayType::Vec(inner_ty.clone()));

            if params.len.is_none() {
                return Err(Error::new_spanned(attr, "missing length parameter"));
            }

            if matches!(&params.len, Some(ArrayLength::Greedy(_)))
                && !matches_ident(&inner_ty, "u8")
            {
                return Err(Error::new_spanned(
                    field,
                    "Greedy arrays must be byte arrays",
                ));
            }
        } else if matches_ident(&inner_ty, "String") {
            is_array = true;
            inner_ty_info = Some(ArrayType::String);

            if params.len.is_none() {
                return Err(Error::new_spanned(attr, "missing length parameter"));
            }
        } else if params.len.is_some() {
            return Err(Error::new_spanned(
                attr,
                "lengths can only be applied to array types",
            ));
        }

        let type_info = match (is_option, is_array) {
            (true, true) => TypeInfo::OptionArray {
                condition: params.condition.unwrap(),
                length: params.len.unwrap(),
                outer: outer_ty,
                inner: inner_ty_info.unwrap(),
            },
            (true, false) => TypeInfo::Option {
                condition: params.condition.unwrap(),
                outer: outer_ty,
                inner: inner_ty,
            },
            (false, true) => TypeInfo::Array {
                length: params.len.unwrap(),
                outer: outer_ty,
                inner: inner_ty_info.unwrap(),
            },
            (false, false) => TypeInfo::Regular(outer_ty),
        };

        dest.push(Field {
            ident: field.ident.clone().unwrap(),
            type_info,
        })
    }

    Ok(dest)
}

struct ParseParams {
    len: Option<ArrayLength>,
    condition: Option<Condition>,
}

impl Default for ParseParams {
    fn default() -> Self {
        Self {
            len: None,
            condition: None,
        }
    }
}

impl ParseParams {
    fn from_length(len: ArrayLength) -> Self {
        Self {
            len: Some(len),
            condition: None,
        }
    }

    fn from_condition(cond: Condition) -> Self {
        Self {
            len: None,
            condition: Some(cond),
        }
    }

    fn merge(&mut self, other: Self) -> Result<()> {
        match (&self.len, other.len) {
            (None, Some(len)) => self.len = Some(len),
            (Some(_), Some(len)) => {
                return Err(Error::new_spanned(len, "Duplicate length parameter"))
            }
            (_, None) => {}
        }

        match (&self.condition, other.condition) {
            (None, Some(condition)) => self.condition = Some(condition),
            (Some(_), Some(condition)) => {
                return Err(Error::new_spanned(
                    condition,
                    "Duplicate condition parameter",
                ))
            }
            (_, None) => {}
        }

        Ok(())
    }
}

impl Parse for ParseParams {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut params = Self {
            len: None,
            condition: None,
        };
        let content;
        parenthesized!(content in input);

        while !content.is_empty() {
            let ident: Ident = content.parse()?;
            let new_param = match ident.to_string().as_str() {
                "len" => {
                    content.parse::<Token![=]>()?;
                    ParseParams::from_length(ArrayLength::Expr(syn::parse_str(
                        &content.parse::<LitStr>()?.value(),
                    )?))
                }

                "fixed_len" => {
                    let bytes;
                    parenthesized!(bytes in content);
                    ParseParams::from_length(ArrayLength::Fixed(bytes.parse()?))
                }

                "len_prefixed" => {
                    let bytes;
                    parenthesized!(bytes in content);
                    ParseParams::from_length(ArrayLength::Prefixed(ident, bytes.parse()?))
                }

                "greedy" => ParseParams::from_length(ArrayLength::Greedy(ident)),

                "condition" => {
                    content.parse::<Token![=]>()?;
                    ParseParams::from_condition(Condition::Expr(syn::parse_str(
                        &content.parse::<LitStr>()?.value(),
                    )?))
                }

                "bool_prefixed" => ParseParams::from_condition(Condition::Prefixed(ident)),

                ident_str => {
                    return Err(Error::new_spanned(
                        ident,
                        &format!("Unknown parameter `{}`", ident_str),
                    ))
                }
            };

            params.merge(new_param)?;
        }

        Ok(params)
    }
}

pub struct Field {
    pub ident: Ident,
    pub type_info: TypeInfo,
}

pub enum Condition {
    Expr(Expr),
    Prefixed(Ident),
}

impl ToTokens for Condition {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Expr(expr) => expr.to_tokens(tokens),
            Self::Prefixed(ident) => ident.to_tokens(tokens),
        }
    }
}

pub enum ArrayLength {
    Expr(Expr),
    Fixed(LitInt),
    Prefixed(Ident, LitInt),
    Greedy(Ident),
}

impl ToTokens for ArrayLength {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Expr(expr) => expr.to_tokens(tokens),
            Self::Fixed(lit) => lit.to_tokens(tokens),
            Self::Prefixed(ident, bytes) => {
                ident.to_tokens(tokens);
                bytes.to_tokens(tokens);
            }
            Self::Greedy(ident) => ident.to_tokens(tokens),
        }
    }
}

pub enum TypeInfo {
    Regular(Type),
    Option {
        condition: Condition,
        outer: Type,
        inner: Type,
    },
    Array {
        length: ArrayLength,
        outer: Type,
        inner: ArrayType,
    },
    OptionArray {
        condition: Condition,
        length: ArrayLength,
        outer: Type,
        inner: ArrayType,
    },
}

pub enum ArrayType {
    Vec(Type),
    String,
}

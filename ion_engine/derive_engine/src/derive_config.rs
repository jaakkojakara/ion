use std::ops::Deref;

use proc_macro::TokenStream;

use quote::quote;
use syn::Type;

pub fn impl_config(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let import = if "ion_engine" == std::env::var("CARGO_PKG_NAME").unwrap() {
        quote!(
            use crate::util::config::*;
        )
    } else {
        quote!(
            use ion_engine::util::config::*;
        )
    };

    match &ast.data {
        syn::Data::Struct(data) => {
            let encode_fields: Vec<_> = data
                .fields
                .iter()
                .map(|field| {
                    let field_name = &field.ident;
                    let field_name_str = format!("{}", field.ident.as_ref().unwrap());
                    quote! {
                        self.#field_name.encode_kv_table([&name_key, #field_name_str].join("").as_str(), table);
                    }
                })
                .collect();

            let decode_field_checks: Vec<_> = data
                .fields
                .iter()
                .map(|field| {
                    let field_name = &field.ident;
                    let field_name_str = format!("{}", field.ident.as_ref().unwrap());
                    // Do this to get just the first part of the type, for example "Vec" of "Vec<u64>"
                    match &field.ty {
                        Type::Path(type_path) => {
                            let field_type = &type_path.path.segments.iter().next().unwrap().ident;
                            quote! {
                               let #field_name = #field_type::decode_kv_table([name_key.as_str(), #field_name_str].join("").as_str(), table)?;
                            }
                        },
                        Type::Reference(type_reference) => {
                            if type_reference.lifetime.as_ref().unwrap().ident == "static" {
                                match &type_reference.elem.deref() {
                                    Type::Path(type_path) => {
                                        let field_type = &type_path.path.segments.iter().next().unwrap().ident;
                                        quote! {
                                           let #field_name = <&'static #field_type>::decode_kv_table([name_key.as_str(), #field_name_str].join("").as_str(), table)?;
                                        }
                                    },
                                    _ => panic!("Invalid type for field {:?}", &field.ident),
                                }
                            } else {
                                panic!("Invalid lifetime (must be static) for field {:?}", &field.ident)
                            }
                        },
                        _ => panic!("Invalid type for field {:?}", &field.ident),
                    }
                })
                .collect();

            let decode_fields: Vec<_> = data
                .fields
                .iter()
                .map(|field| {
                    let field_name = &field.ident;
                    quote! {
                        #field_name,
                    }
                })
                .collect();

            let output = quote! {
                #import
                impl Config for #name {
                    fn encode_kv_table(&self, path: &str, table: &mut std::collections::BTreeMap<String, String>) {
                        let name_key = if !path.is_empty() { format!("{}.", path) } else { "".to_owned()  };
                        #( #encode_fields )*
                    }

                    fn decode_kv_table(path: &str, table: &std::collections::BTreeMap<String, String>) -> Result<Self, ConfigParseError>
                    where
                        Self: Sized,
                    {
                        let name_key = if !path.is_empty() {
                            format!("{}.", path)
                        } else {
                            "".to_owned()
                        };

                        #( #decode_field_checks )*

                        Ok(Self { #( #decode_fields )* })
                    }
                }
            };
            output.into()
        }
        syn::Data::Enum(data) => {
            if data.variants.iter().any(|var| !var.fields.is_empty()) {
                panic!("Config can be derived only for enums with no fields");
            }

            let encode_fields: Vec<_> = data
                .variants
                .iter()
                .map(|variant| {
                    let field_name = &variant.ident;
                    let field_name_str = format!("\"{}\"", variant.ident);
                    quote! {
                        #name::#field_name => table.insert(path.to_owned(), #field_name_str.to_owned()),
                    }
                })
                .collect();

            let decode_fields: Vec<_> = data
                .variants
                .iter()
                .map(|variant| {
                    let field_name = &variant.ident;
                    let field_name_str = format!("\"{}\"", variant.ident);
                    quote! {
                        #field_name_str => Ok(Self::#field_name),
                    }
                })
                .collect();

            let output = quote! {
                #import
                impl Config for #name {
                    fn encode_kv_table(&self, path: &str, table: &mut std::collections::BTreeMap<String, String>) {
                        match self {
                            #( #encode_fields )*
                        };
                    }

                    fn decode_kv_table(path: &str, table: &std::collections::BTreeMap<String, String>) -> Result<Self, ConfigParseError>
                    where
                        Self: Sized,
                    {
                        if let Some(value) = table.get(path) {
                            match value.as_str() {
                                #( #decode_fields )*
                                _ => Err(ConfigParseError::InvalidFieldType(format!("{} = {}", path, value))),
                            }
                        } else {
                            Err(ConfigParseError::MissingData(format!("Missing field: {}", path)))
                        }
                    }
                }
            };
            output.into()
        }
        syn::Data::Union(_) => panic!("Config can't be applied to unions"),
    }
}

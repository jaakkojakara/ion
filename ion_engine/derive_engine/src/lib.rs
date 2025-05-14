use derive_config::impl_config;
use derive_raw_data::impl_raw_data;
use proc_macro::TokenStream;

mod derive_config;
mod derive_raw_data;

/// Derives a Config implementation for the given struct.
/// There are some limitations on which types can automatically derive Config:
/// - Structs can only contain fields that implement Config
/// - Enums can only contain fields that do not contain any inner values
/// - Unions are not supported
///
/// If this macro fails to derive Config implementation, it can be manually implemented.
#[proc_macro_derive(Config)]
pub fn config_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_config(&ast)
}

/// Derives a RawData implementation for the given struct.
///
/// ### Safety
/// Deriving RawData with this does not guarantee that it is safe to implement RawData. This macro only check for the following:
/// - Object is struct and not an enum or union
/// - Object has no padding bytes
/// - All the fields in the object also implement RawData
#[proc_macro_derive(RawData)]
pub fn raw_data_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_raw_data(&ast)
}

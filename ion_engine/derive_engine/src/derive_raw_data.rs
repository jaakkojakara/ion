use proc_macro::TokenStream;
use quote::{quote, quote_spanned};

pub fn impl_raw_data(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let span = ast.ident.span();

    match &ast.data {
        syn::Data::Struct(data) => {
            let fields = &data.fields;
            let mut field_types = fields.iter().map(|field| &field.ty);
            let size_sum = if let Some(first) = field_types.next() {
                let size_first = quote_spanned!(span => ::core::mem::size_of::<#first>());
                let size_rest = quote_spanned!(span => #( + ::core::mem::size_of::<#field_types>() )*);

                quote_spanned!(span => #size_first #size_rest)
            } else {
                quote_spanned!(span => 0)
            };

            let padding_check = quote_spanned! {span => const _: fn() = || {
              #[doc(hidden)]
              #[allow(dead_code)]
              struct TypeWithoutPadding([u8; #size_sum]);
              let _ = ::core::mem::transmute::<#name, TypeWithoutPadding>;
            };};

            let fields_checks: Vec<_> = fields
                .iter()
                .map(|field| {
                    let field_name = &field.ident;
                    quote! {
                        Self::field_has_raw_data(&self.#field_name);
                    }
                })
                .collect();
            let output = quote! {
                impl #name {
                    fn field_has_raw_data(field: &impl RawData) {}
                    fn bounds_check(&self) {
                        #( #fields_checks )*
                    }
                    fn padding_check(&self) {
                        #padding_check
                    }
                }

                unsafe impl RawData for #name {}
            };
            output.into()
        }
        syn::Data::Enum(_) | syn::Data::Union(_) => {
            panic!("RawData can't be applied to enums or unions")
        }
    }
}

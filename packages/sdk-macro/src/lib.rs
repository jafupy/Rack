mod cron;
mod metadata;
mod route;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

#[proc_macro_attribute]
pub fn payload(_: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemStruct);
    let ident = &item.ident;
    let generics = &item.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        #[derive(rack::serde::Serialize, rack::serde::Deserialize)]
        #[serde(crate = "rack::serde")]
        #item

        impl #impl_generics rack::Payload for #ident #ty_generics #where_clause {
            fn from_body(body: &[u8]) -> rack::Result<Self> {
                rack::__private::payload_from_json(body)
            }

            fn into_body(self) -> rack::Result<Vec<u8>> {
                rack::__private::payload_to_json(self)
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn route(args: TokenStream, item: TokenStream) -> TokenStream {
    route::expand(args.into(), item.into()).into()
}

#[proc_macro_attribute]
pub fn cron(args: TokenStream, item: TokenStream) -> TokenStream {
    cron::expand(args.into(), item.into()).into()
}

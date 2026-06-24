use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse2, ItemFn, LitStr, Result};

use crate::metadata;

pub fn expand(args: TokenStream, item: TokenStream) -> TokenStream {
    match expand_result(args, item) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

fn expand_result(args: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let schedule: LitStr = parse2(args)?;
    let function: ItemFn = parse2(item)?;
    let name = &function.sig.ident;
    let wrapper = format_ident!("__rack_cron_{name}");
    let entry = wrapper.to_string();
    let meta = metadata::cron(name, &schedule.value(), &entry);

    Ok(quote! {
        #function

        #[export_name = #entry]
        pub extern "C" fn #wrapper() {
            #name()
        }

        #meta
    })
}

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{parse2, Error, Ident, ItemFn, LitStr, Result, Token};

use crate::metadata;

struct RouteArgs {
    method: Ident,
    path: LitStr,
}

impl Parse for RouteArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let method = input.parse()?;
        input.parse::<Token![,]>()?;
        let path = input.parse()?;
        Ok(Self { method, path })
    }
}

pub fn expand(args: TokenStream, item: TokenStream) -> TokenStream {
    match expand_result(args, item) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

fn expand_result(args: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let args: RouteArgs = parse2(args)?;
    let function: ItemFn = parse2(item)?;
    validate_signature(&function)?;

    let name = &function.sig.ident;
    let wrapper = format_ident!("__rack_route_{name}");
    let entry = wrapper.to_string();
    let method = args.method.to_string();
    let meta = metadata::http(name, &method, &args.path.value(), &entry);

    Ok(quote! {
        #function

        #[export_name = #entry]
        pub extern "C" fn #wrapper(req_ptr: i32, req_len: i32) -> i64 {
            rack::__private::run_http(#name, req_ptr, req_len)
        }

        #meta
    })
}

fn validate_signature(function: &ItemFn) -> Result<()> {
    if function.sig.asyncness.is_some() {
        return Err(Error::new_spanned(
            &function.sig.asyncness,
            "routes cannot be async yet",
        ));
    }
    Ok(())
}

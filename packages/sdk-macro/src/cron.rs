use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse2, Error, FnArg, ItemFn, LitStr, Pat, Result};

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
    validate_signature(&function)?;
    let name = &function.sig.ident;
    let wrapper = format_ident!("__rack_cron_{name}");
    let entry = wrapper.to_string();
    let meta = metadata::cron(name, &schedule.value(), &entry);
    let call = match function.sig.inputs.len() {
        0 => quote! { #name() },
        1 => quote! { #name(rack::__private::read_cron_event(event_ptr, event_len)) },
        _ => unreachable!("cron signature was validated"),
    };

    Ok(quote! {
        #function

        #[export_name = #entry]
        pub extern "C" fn #wrapper(event_ptr: i32, event_len: i32) {
            #call
        }

        #meta
    })
}

fn validate_signature(function: &ItemFn) -> Result<()> {
    if function.sig.asyncness.is_some() {
        return Err(Error::new_spanned(
            &function.sig.asyncness,
            "cron hooks cannot be async yet",
        ));
    }

    if function.sig.inputs.len() > 1 {
        return Err(Error::new_spanned(
            &function.sig.inputs,
            "cron hooks accept either no arguments or one CronEvent argument",
        ));
    }

    if let Some(FnArg::Receiver(receiver)) = function.sig.inputs.first() {
        return Err(Error::new_spanned(receiver, "cron hooks cannot take self"));
    }

    if let Some(FnArg::Typed(arg)) = function.sig.inputs.first() {
        if !matches!(arg.pat.as_ref(), Pat::Ident(_)) {
            return Err(Error::new_spanned(
                &arg.pat,
                "cron hook argument must be an identifier",
            ));
        }
    }

    Ok(())
}

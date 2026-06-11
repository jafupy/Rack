//! Proc macros re-exported by the `rack` SDK crate.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, AngleBracketedGenericArguments, FnArg, GenericArgument, ItemFn, ItemStruct,
    PathArguments, ReturnType, Stmt, Type,
};

/// Implement `rack::Payload` for a JSON request/response body struct.
///
/// The generated impl uses `rack::serde` so function crates only need a
/// dependency on `rack`.
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

/// Export a Rust function as a Rack HTTP route handler.
///
/// The manifest still owns the route path and method. This macro only creates
/// the no-argument C ABI wrapper expected by Rack.
#[proc_macro_attribute]
pub fn route(_: TokenStream, item: TokenStream) -> TokenStream {
    expand_handler(parse_macro_input!(item as ItemFn), HandlerKind::Route).into()
}

/// Export a Rust function as a Rack scheduled handler.
///
/// The manifest still owns the cron schedule. This macro only creates the
/// no-argument C ABI wrapper expected by Rack.
#[proc_macro_attribute]
pub fn cron(_: TokenStream, item: TokenStream) -> TokenStream {
    expand_handler(parse_macro_input!(item as ItemFn), HandlerKind::Cron).into()
}

enum HandlerKind {
    Route,
    Cron,
}

fn expand_handler(mut item: ItemFn, kind: HandlerKind) -> proc_macro2::TokenStream {
    let name = item.sig.ident.clone();
    let inner_name = format_ident!("__rack_inner_{}", name);
    let arg = match item.sig.inputs.len() {
        0 => None,
        1 => item.sig.inputs.first().cloned(),
        _ => {
            return quote! {
                compile_error!("rack handlers accept zero args or one rack::Request<T>/rack::CronEvent arg");
            };
        }
    };

    if !returns_response(&item.sig.output) {
        return quote! {
            compile_error!("rack handlers must return rack::Response");
        };
    }

    if let Err(error) = wrap_tail_expression(&mut item) {
        return quote! {
            compile_error!(#error);
        };
    }

    let block = item.block;
    let runner = match (kind, arg.clone()) {
        (HandlerKind::Route, None) => quote! {
            fn #inner_name() -> rack::__private::HandlerResult<rack::Response> #block
            rack::__private::run_route_empty(#inner_name);
        },
        (HandlerKind::Route, Some(FnArg::Typed(arg))) => {
            let Some(body_type) = request_body_type(&arg.ty) else {
                return quote! {
                    compile_error!("route handlers take rack::Request<T> or no args");
                };
            };
            quote! {
                fn #inner_name(#arg) -> rack::__private::HandlerResult<rack::Response> #block
                rack::__private::run_route::<#body_type, _>(#inner_name);
            }
        }
        (HandlerKind::Route, Some(FnArg::Receiver(_))) => quote! {
            compile_error!("rack handlers cannot take self");
        },
        (HandlerKind::Cron, None) => quote! {
            fn #inner_name() -> rack::__private::HandlerResult<rack::Response> #block
            rack::__private::run_cron_empty(#inner_name);
        },
        (HandlerKind::Cron, Some(FnArg::Typed(arg))) => {
            if !is_cron_event_type(&arg.ty) {
                return quote! {
                    compile_error!("cron handlers take rack::CronEvent or no args");
                };
            }
            quote! {
                fn #inner_name(#arg) -> rack::__private::HandlerResult<rack::Response> #block
                rack::__private::run_cron(#inner_name);
            }
        }
        (HandlerKind::Cron, Some(FnArg::Receiver(_))) => quote! {
            compile_error!("rack handlers cannot take self");
        },
    };

    quote! {
        #[no_mangle]
        pub extern "C" fn #name() {
            #runner
        }
    }
}

fn wrap_tail_expression(item: &mut ItemFn) -> Result<(), &'static str> {
    let Some(last) = item.block.stmts.pop() else {
        return Err("rack handlers must end with a rack::Response expression");
    };

    match last {
        Stmt::Expr(expr, None) => {
            item.block.stmts.push(syn::parse_quote! {
                return Ok(#expr);
            });
            Ok(())
        }
        _ => Err("rack handlers must end with a rack::Response expression"),
    }
}

fn returns_response(output: &ReturnType) -> bool {
    let ReturnType::Type(_, ty) = output else {
        return false;
    };

    last_type_segment(ty).is_some_and(|segment| segment == "Response")
}

fn request_body_type(ty: &Type) -> Option<proc_macro2::TokenStream> {
    let Type::Path(path) = ty else {
        return None;
    };

    let segment = path.path.segments.last()?;
    if segment.ident != "Request" {
        return None;
    }

    match &segment.arguments {
        PathArguments::None => Some(quote! { () }),
        PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) => {
            match args.first()? {
                GenericArgument::Type(ty) => Some(quote! { #ty }),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_cron_event_type(ty: &Type) -> bool {
    last_type_segment(ty).is_some_and(|segment| segment == "CronEvent")
}

fn last_type_segment(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };

    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

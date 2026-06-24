mod cron;
mod metadata;
mod route;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn route(args: TokenStream, item: TokenStream) -> TokenStream {
    route::expand(args.into(), item.into()).into()
}

#[proc_macro_attribute]
pub fn cron(args: TokenStream, item: TokenStream) -> TokenStream {
    cron::expand(args.into(), item.into()).into()
}

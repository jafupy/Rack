use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

pub fn http(function: &Ident, method: &str, path: &str, entry: &str) -> TokenStream {
    let json = serde_json::json!({
        "kind": "http",
        "id": function.to_string(),
        "method": method,
        "path": normalize_path(path),
        "entry": entry,
    });
    static_tokens(function, "ROUTE", &json.to_string())
}

pub fn cron(function: &Ident, schedule: &str, entry: &str) -> TokenStream {
    let json = serde_json::json!({
        "kind": "cron",
        "id": function.to_string(),
        "schedule": schedule,
        "entry": entry,
    });
    static_tokens(function, "CRON", &json.to_string())
}

fn static_tokens(function: &Ident, suffix: &str, json: &str) -> TokenStream {
    let name = format_ident!("__RACK_{}_{}", suffix, function.to_string().to_uppercase());
    let data = format!("{json}\n");
    let len = data.len();
    let bytes = Literal::byte_string(data.as_bytes());
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[link_section = "rack.hooks"]
        #[used]
        static #name: [u8; #len] = *#bytes;
    }
}

fn normalize_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_path;

    #[test]
    fn prefixes_route_paths() {
        assert_eq!(normalize_path("gscse"), "/gscse");
        assert_eq!(normalize_path("/gscse"), "/gscse");
    }
}

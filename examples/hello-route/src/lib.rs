use std::io::{self, Read};

#[no_mangle]
pub extern "C" fn hello() {
    let mut request = String::new();
    let _ = io::stdin().read_to_string(&mut request);

    let user = std::env::var("USER").unwrap_or_else(|_| "there".to_string());
    let body = if request.trim().is_empty() {
        "Rack handed this function an empty request.".to_string()
    } else {
        format!("Rack handed this function {} request bytes.", request.len())
    };

    println!("hello, {user}");
    println!("{body}");
}


//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

// use push_page_diff::push::create_vapid_jwt;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn pass() {
    assert_eq!(1 + 1, 2);
}

// #[wasm_bindgen_test]
// fn test_message() {
//     let token = create_vapid_jwt(
//             "".to_string(),
//             "mailto:test@example.com".to_string(),
//         ).unwrap();
//     assert_eq!(token, "".to_string())
// }

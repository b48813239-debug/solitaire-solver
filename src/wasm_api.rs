//! Liaison WebAssembly — échange en JSON.

use wasm_bindgen::prelude::*;

use crate::determinize::{solve_one_world, KnownState};

#[wasm_bindgen]
pub fn wasm_init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn wasm_solve_one_world(known_json: &str, world_seed: u64, budget: u64) -> String {
    let known: KnownState = match serde_json::from_str(known_json) {
        Ok(k) => k,
        Err(e) => return format!("{{\"error\":\"KnownState JSON invalide: {}\"}}", e.to_string().replace('"', "'")),
    };
    let outcome = solve_one_world(&known, world_seed, budget);
    serde_json::to_string(&outcome).expect("sérialisation de WorldOutcome infaillible pour ce type")
}

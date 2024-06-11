// @generated by autocargo

use std::env;
use std::fs;
use std::path::Path;
use thrift_compiler::Config;
use thrift_compiler::GenContext;
const CRATEMAP: &str = "\
common/rust/shed/justknobs_stub/cached_config_thrift_struct/just_knobs.thrift crate //common/rust/shed/justknobs_stub/cached_config_thrift_struct:just_knobs_struct-rust
";
#[rustfmt::skip]
fn main() {
    println!("cargo:rerun-if-changed=thrift_build.rs");
    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR env not provided");
    let cratemap_path = Path::new(&out_dir).join("cratemap");
    fs::write(cratemap_path, CRATEMAP).expect("Failed to write cratemap");
    Config::from_env(GenContext::Types)
        .expect("Failed to instantiate thrift_compiler::Config")
        .base_path("../../../../..")
        .types_crate("just_knobs_struct__types")
        .clients_crate("just_knobs_struct__clients")
        .options("serde")
        .run(["just_knobs.thrift"])
        .expect("Failed while running thrift compilation");
}

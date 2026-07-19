fn main() {
    println!("cargo::rerun-if-env-changed=RAILWAY_GIT_COMMIT_SHA");
    build_info_build::build_script();
}

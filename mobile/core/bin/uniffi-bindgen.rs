// In-crate UniFFI bindings generator. Pinned to this crate's exact UniFFI
// version so generated Kotlin/Swift always match the compiled scaffolding,
// regardless of any globally-installed `uniffi-bindgen`. Invoked in library
// mode by build-ios.sh / build-android.sh against the compiled dylib.
fn main() {
    uniffi::uniffi_bindgen_main()
}

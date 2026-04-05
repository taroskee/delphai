pub mod llm;

#[cfg(feature = "godot-extension")]
mod godot_entry {
    use godot::prelude::*;

    struct DelphaiExtension;

    #[gdextension]
    unsafe impl ExtensionLibrary for DelphaiExtension {}
}

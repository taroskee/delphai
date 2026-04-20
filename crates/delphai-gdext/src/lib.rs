use godot::prelude::*;

struct DelphaiExtension;

#[gdextension]
unsafe impl ExtensionLibrary for DelphaiExtension {}

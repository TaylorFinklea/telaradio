# model-adapter/

Rust crate. Implements the `Generator` trait by spawning and managing a
Python subprocess running ACE-Step 1.5 XL.

The trait is defined in `core/`:

```rust
pub trait Generator {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn generate(&self, prompt: &str, seed: u64, duration: u32)
        -> Result<WavBuffer, GeneratorError>;
}
```

This crate ships with one implementation (`AceStepGenerator`). Future
generators (MusicGen, YuE, ...) live alongside it, and recipes pin
`model.id + model.version` to lock down which one runs.

First-launch model download (resumable HTTP from Hugging Face into
`~/Library/Application Support/Lockstep/models/`) lives here too.

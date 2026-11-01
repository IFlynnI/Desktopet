# The OS boundary is a Bevy-free crate with one folder per OS

Windows is the only target today, but macOS and Linux are on the Platform track. All operating-system code
therefore lives in `pet-platform`, which doesn't depend on Bevy. Its `lib.rs` is an OS-neutral API, and `#[cfg]`
picks one implementation folder, such as `src/windows/`, at compile time. Adding an OS adds a folder and moves
nothing.

We chose a crate over a module in `pet-app` because a module boundary is only a convention, while a crate makes
the compiler reject Bevy in platform code. We chose `#[cfg]` over a Rust trait because only one implementation
exists per build, and the app talks to the platform through plain types and channels, so tests can inject fake
events without a fake platform. Only the public API is OS-neutral for now. Internals such as the hit-test thread
stay in `src/windows/` until a second OS shows what's actually shared.

Two pieces of OS-specific code stay outside the crate: choosing the DX12 backend is a Bevy setting, and the
end-to-end harness tests the real exe from `pet-app`.

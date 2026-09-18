//! Stardrop entry point. All game code lives in the library crate (`lib.rs`)
//! so that benchmarks and integration tests can use it too.

fn main() -> bevy::app::AppExit {
    stardrop::run()
}

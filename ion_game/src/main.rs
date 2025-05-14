use ion_game::run;

/// Native target entry point for the game.
/// Wasm calls the `run` function directly from JS side.
fn main() {
    run();
}

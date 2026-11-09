//! `ranuka` binary: headless commands and the TUI.
//!
//! Uses the public `ranuka-sdk` only. Anything missing here is missing in the SDK.

fn main() {
    println!("ranuka {}", env!("CARGO_PKG_VERSION"));
}

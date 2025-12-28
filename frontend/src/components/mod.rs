pub mod board;
pub mod timeline;

/// Convert a bool to a float, which is useful for CSS opacity
pub fn b2f(b: bool) -> f32 {
    if b { 1.0 } else { 0.0 }
}

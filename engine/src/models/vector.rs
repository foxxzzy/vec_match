/// Thin alias used to clarify intent when a `Vec<f32>` represents an embedding/vector.
///
/// This keeps storage as a plain `Vec<f32>` while making APIs self-documenting.
pub type Vector = Vec<f32>;

pub mod hash;
pub mod matrix;
pub mod rand;

/// Performs smooth Hermite interpolation between 0 and 1 when `val` is between `from` and `to`.
///
/// # Parameters
/// - `from`: The lower bound of the interpolation range.
/// - `to`: The upper bound of the interpolation range.
/// - `val`: The value to interpolate.
///
/// # Returns
/// The interpolated value, smoothly transitioning from 0 to 1 as `val` moves from `from` to `to`.
#[inline]
pub fn smoothstep(from: f32, to: f32, val: f32) -> f32 {
    let x = clamp((val - from) / (to - from), 0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Clamps a value between a lower and upper limit.
///
/// # Parameters
/// - `val`: The value to clamp.
/// - `lower_limit`: The minimum allowed value.
/// - `upper_limit`: The maximum allowed value.
///
/// # Returns
/// The clamped value, guaranteed to be between `lower_limit` and `upper_limit`.
#[inline]
pub fn clamp(val: f32, lower_limit: f32, upper_limit: f32) -> f32 {
    val.max(lower_limit).min(upper_limit)
}

/// Linearly interpolates between two values.
///
/// # Parameters
/// - `from`: The start value.
/// - `to`: The end value.
/// - `t`: The interpolation factor, typically between 0.0 and 1.0.
///
/// # Returns
/// The interpolated value between `from` and `to` at position `t`.
#[inline]
pub fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + t * (to - from)
}

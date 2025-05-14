/// A marker trait for "raw data" types that can be safely cast to byte slices.
/// Main use is to cast a type to ```&[u8]``` for transferring it to GPU
///
/// ### Safety
/// - Type must be a struct
/// - Type must be instantiable
/// - Type must not contain padding
/// - All bit patterns must be valid
/// - Type cannot contain pointers or interior mutability
pub unsafe trait RawData: Copy + 'static {}

unsafe impl RawData for u8 {}

unsafe impl RawData for u16 {}

unsafe impl RawData for u32 {}

unsafe impl RawData for u64 {}

unsafe impl RawData for i8 {}

unsafe impl RawData for i16 {}

unsafe impl RawData for i32 {}

unsafe impl RawData for i64 {}

unsafe impl RawData for f32 {}

unsafe impl RawData for f64 {}

unsafe impl RawData for usize {}

unsafe impl RawData for isize {}

unsafe impl<T: RawData, const N: usize> RawData for [T; N] {}

pub fn slice_as_bytes<T: RawData>(from: &[T]) -> &[u8] {
    unsafe { core::slice::from_raw_parts(from.as_ptr() as *const u8, core::mem::size_of_val(from)) }
}

pub fn any_as_bytes<T: RawData>(from: &T) -> &[u8] {
    unsafe { core::slice::from_raw_parts((from as *const T) as *const u8, core::mem::size_of::<T>()) }
}

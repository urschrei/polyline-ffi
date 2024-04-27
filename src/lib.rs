#![doc(
    html_logo_url = "https://cdn.rawgit.com/urschrei/polyline-ffi/master/line.svg",
    html_root_url = "https://docs.rs/polyline-ffi/"
)]
//! This module exposes functions for accessing the Polyline encoding and decoding functions via FFI
//!
//!
//! ## A Note on Coordinate Order
//! This crate uses `Coordinate` and `LineString` types from the `geo-types` crate, which encodes coordinates in `(x, y)` order. The Polyline algorithm and first-party documentation assumes the _opposite_ coordinate order. It is thus advisable to pay careful attention to the order of the coordinates you use for encoding and decoding.

#![deny(
    clippy::cast_slice_from_raw_parts,
    clippy::cast_slice_different_sizes,
    clippy::invalid_null_ptr_usage,
    clippy::ptr_as_ptr,
    clippy::transmute_ptr_to_ref
)]

use polyline::{decode_polyline, encode_coordinates};
use std::ffi::{CStr, CString};
use std::slice;
use std::{f64, ptr};

use geo_types::{CoordFloat, LineString};
use libc::c_char;

// we only want to allow 5 or 6, but we need the previous values for the cast to work
#[allow(dead_code)]
enum Precision {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
}

// We currently only allow 5 or 6
fn get_precision(input: u32) -> Option<u32> {
    match input {
        5 => Some(Precision::Five as u32),
        6 => Some(Precision::Six as u32),
        _ => None,
    }
}

/// A C-compatible `struct` originating **outside** Rust
/// used for passing arrays across the FFI boundary
#[repr(C)]
pub struct ExternalArray {
    pub data: *const libc::c_void,
    pub len: libc::size_t,
}

/// A C-compatible `struct` originating **inside** Rust
/// used for passing arrays across the FFI boundary
#[repr(C)]
pub struct InternalArray {
    pub data: *mut libc::c_void,
    pub len: libc::size_t,
}

impl Drop for InternalArray {
    fn drop(&mut self) {
        if self.data.is_null() {
            return;
        }
        unsafe {
            // we originated this data, so pointer-to-slice -> box -> vec
            let p = ptr::slice_from_raw_parts_mut(self.data.cast::<[f64; 2]>(), self.len);
            drop(Box::from_raw(p));
        };
    }
}

// Build an InternalArray from a LineString, so it can be leaked across the FFI boundary
impl<T> From<LineString<T>> for InternalArray
where
    T: CoordFloat,
{
    fn from(sl: LineString<T>) -> Self {
        let v: Vec<[T; 2]> = sl.0.iter().map(|p| [p.x, p.y]).collect();
        let boxed = v.into_boxed_slice();
        let blen = boxed.len();
        let rawp = Box::into_raw(boxed);
        InternalArray {
            data: rawp.cast::<libc::c_void>(),
            len: blen as libc::size_t,
        }
    }
}

// Build a LineString from an InternalArray
impl From<InternalArray> for LineString<f64> {
    fn from(arr: InternalArray) -> Self {
        // we originated this data, so pointer-to-slice -> box -> vec
        unsafe {
            let p = ptr::slice_from_raw_parts_mut(arr.data.cast::<[f64; 2]>(), arr.len);
            let v = Box::from_raw(p).to_vec();
            v.into()
        }
    }
}

// Build an InternalArray from a LineString, so it can be leaked across the FFI boundary
impl From<Vec<[f64; 2]>> for InternalArray {
    fn from(v: Vec<[f64; 2]>) -> Self {
        let boxed = v.into_boxed_slice();
        let blen = boxed.len();
        let rawp = Box::into_raw(boxed);
        InternalArray {
            data: rawp.cast::<libc::c_void>(),
            len: blen as libc::size_t,
        }
    }
}

// Build an InternalArray from a LineString, so it can be leaked across the FFI boundary
impl From<Vec<[f64; 2]>> for ExternalArray {
    fn from(v: Vec<[f64; 2]>) -> Self {
        let boxed = v.into_boxed_slice();
        let blen = boxed.len();
        let rawp = Box::into_raw(boxed);
        ExternalArray {
            data: rawp.cast::<libc::c_void>(),
            len: blen as libc::size_t,
        }
    }
}

// Build a LineString from an ExternalArray
impl From<ExternalArray> for LineString<f64> {
    fn from(arr: ExternalArray) -> Self {
        // we need to take ownership of this data, so slice -> vec
        unsafe {
            let v = slice::from_raw_parts(arr.data as *mut [f64; 2], arr.len).to_vec();
            v.into()
        }
    }
}

// Decode a Polyline into an InternalArray
fn arr_from_string(incoming: &str, precision: u32) -> InternalArray {
    let result: InternalArray = if get_precision(precision).is_some() {
        match decode_polyline(incoming, precision) {
            Ok(res) => res.into(),
            // should be easy to check for
            Err(_) => vec![[f64::NAN, f64::NAN]].into(),
        }
    } else {
        // bad precision parameter
        vec![[f64::NAN, f64::NAN]].into()
    };
    result
}

// Decode an Array into a Polyline
fn string_from_arr(incoming: ExternalArray, precision: u32) -> String {
    let inc: LineString<_> = incoming.into();
    if get_precision(precision).is_some() {
        match encode_coordinates(Into::<LineString<_>>::into(inc), precision) {
            Ok(res) => res,
            // we don't need to adapt the error
            Err(res) => res,
        }
    } else {
        "Bad precision parameter supplied".to_string()
    }
}

/// Convert a Polyline into an array of coordinates
///
/// Callers must pass two arguments:
///
/// - a pointer to `NUL`-terminated characters (`char*`)
/// - an unsigned 32-bit `int` for precision (5 for Google Polylines, 6 for
/// OSRM and Valhalla Polylines)
///
/// A decoding failure will return an [Array](struct.Array.html) whose `data` field is `[[NaN, NaN]]`, and whose `len` field is `1`.
///
/// Implementations calling this function **must** call [`drop_float_array`](fn.drop_float_array.html)
/// with the returned [Array](struct.Array.html), in order to free the memory it allocates.
///
/// # Safety
///
/// This function is unsafe because it accesses a raw pointer which could contain arbitrary data
#[no_mangle]
pub unsafe extern "C" fn decode_polyline_ffi(pl: *const c_char, precision: u32) -> InternalArray {
    let s = CStr::from_ptr(pl).to_str();
    if let Ok(unwrapped) = s {
        arr_from_string(unwrapped, precision)
    } else {
        vec![[f64::NAN, f64::NAN]].into()
    }
}

/// Convert an array of coordinates into a Polyline
///
/// Callers must pass two arguments:
///
/// - a [Struct](struct.Array.html) with two fields:
///     - `data`, a void pointer to an array of floating-point lat, lon coordinates: `[[1.0, 2.0]]`
///     - `len`, the length of the array being passed. Its type must be `size_t`: `1`
/// - an unsigned 32-bit `int` for precision (5 for Google Polylines, 6 for
/// OSRM and Valhalla Polylines)
///
/// A decoding failure will return one of the following:
///
/// - a `char*` beginning with "Longitude error:" if invalid longitudes are passed
/// - a `char*` beginning with "Latitude error:" if invalid latitudes are passed
///
/// Implementations calling this function **must** call [`drop_cstring`](fn.drop_cstring.html)
/// with the returned `c_char` pointer, in order to free the memory it allocates.
///
/// # Safety
///
/// This function is unsafe because it accesses a raw pointer which could contain arbitrary data
#[no_mangle]
pub extern "C" fn encode_coordinates_ffi(coords: ExternalArray, precision: u32) -> *mut c_char {
    let s: String = string_from_arr(coords, precision);
    match CString::new(s) {
        Ok(res) => res.into_raw(),
        // It's arguably better to fail noisily, but this is robust
        Err(_) => CString::new("Couldn't decode Polyline".to_string())
            .unwrap()
            .into_raw(),
    }
}

/// Free Array memory which Rust has allocated across the FFI boundary
///
/// # Safety
///
/// This function is unsafe because it accesses a raw pointer which could contain arbitrary data
#[no_mangle]
pub extern "C" fn drop_float_array(_: InternalArray) {}

/// Free `CString` memory which Rust has allocated across the FFI boundary
///
/// # Safety
///
/// This function is unsafe because it accesses a raw pointer which could contain arbitrary data
#[no_mangle]
pub unsafe extern "C" fn drop_cstring(p: *mut c_char) {
    drop(CString::from_raw(p));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn test_drop_empty_float_array() {
        let original: LineString<_> = vec![[2.0, 1.0], [4.0, 3.0]].into();
        // move into an Array, and leak it
        let mut arr: InternalArray = original.into();
        // zero Array contents
        arr.data = ptr::null_mut();
        drop_float_array(arr);
    }

    #[test]
    fn test_coordinate_conversion() {
        let input = vec![[2.0, 1.0], [4.0, 3.0]];
        let output = "_ibE_seK_seK_seK";
        let input_arr: ExternalArray = input.into();
        let transformed: String = super::string_from_arr(input_arr, 5);
        assert_eq!(transformed, output);
    }

    #[test]
    fn test_string_conversion() {
        let input = "_ibE_seK_seK_seK";
        let output = vec![[2.0, 1.0], [4.0, 3.0]];
        // String to Array
        let transformed: InternalArray = super::arr_from_string(input, 5);
        // Array to LS via slice, as we want to take ownership of a copy for testing purposes
        let v = unsafe {
            slice::from_raw_parts(transformed.data as *mut [f64; 2], transformed.len).to_vec()
        };
        let ls: LineString<_> = v.into();
        assert_eq!(ls, output.into());
    }

    #[test]
    #[should_panic]
    fn test_bad_string_conversion() {
        let input = "_p~iF~ps|U_u🗑lLnnqC_mqNvxq`@";
        let output = vec![[1.0, 2.0], [3.0, 4.0]];
        // String to Array
        let transformed: InternalArray = super::arr_from_string(input, 5);
        // Array to LS via slice, as we want to take ownership of a copy for testing purposes
        let v = unsafe {
            slice::from_raw_parts(transformed.data as *mut [f64; 2], transformed.len).to_vec()
        };
        let ls: LineString<_> = v.into();
        assert_eq!(ls, output.into());
    }

    #[test]
    fn test_long_vec() {
        use std::clone::Clone;
        let arr = include!("../test_fixtures/berlin.rs");
        let s = include!("../test_fixtures/berlin_decoded.rs");
        for _ in 0..9999 {
            let a = arr.clone();
            let n = 5;
            let input_ls: ExternalArray = a.into();
            let transformed: String = super::string_from_arr(input_ls, n);
            assert_eq!(transformed, s);
        }
    }
}

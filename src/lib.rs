#![doc(html_logo_url = "https://cdn.rawgit.com/urschrei/polyline-ffi/master/line.svg",
       html_root_url = "https://urschrei.github.io/polyline-ffi/")]
//! This module exposes functions for accessing the Polyline encoding and decoding functions via FFI
extern crate polyline;
use polyline::{encode_coordinates, decode_polyline};
use std::mem;
use std::slice;
use std::f64;
use std::ffi::{CStr, CString};

extern crate libc;
use self::libc::{c_char, c_void, uint32_t, size_t};

#[repr(C)]
#[derive(Clone)]
pub struct Array {
    pub data: *const c_void,
    pub len: size_t,
}

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
fn get_precision(input: &uint32_t) -> Option<uint32_t> {
    match *input {
        5 => Some(Precision::Five as uint32_t),
        6 => Some(Precision::Six as uint32_t),
        _ => None,
    }
}

// Build an Array from &[[f64; 2]], so it can be leaked across the FFI boundary
impl From<Vec<[f64; 2]>> for Array {
    fn from(sl: Vec<[f64; 2]>) -> Self {
        let array = Array {
            data: sl.as_ptr() as *const c_void,
            len: sl.len() as size_t,
        };
        mem::forget(sl);
        array
    }
}

// Build &[[f64; 2]] from an Array, so it can be dropped
impl From<Array> for Vec<[f64; 2]> {
    fn from(arr: Array) -> Self {
        unsafe { slice::from_raw_parts(arr.data as *mut [f64; 2], arr.len).to_vec() }
    }
}

// Decode a Polyline into an Array
fn arr_from_string(incoming: &str, precision: uint32_t) -> Array {
    let result: Array = if get_precision(&precision).is_some() {
        match decode_polyline(incoming, precision) {
            Ok(res) => res.into(),
            // should be easy to check for
            Err(_) => vec![[f64::NAN, f64::NAN]].into(),
        }
    } else {
        // bad precision parameter
        vec![[f64::NAN, f64::NAN]].into()
    };
    result.into()
}

// Decode an Array into a Polyline
fn string_from_arr(incoming: Array, precision: uint32_t) -> String {
    let inc: Vec<_> = incoming.into();
    if get_precision(&precision).is_some() {
        match encode_coordinates(&inc, precision) {
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
pub extern "C" fn decode_polyline_ffi(pl: *const c_char, precision: uint32_t) -> Array {
    let s = unsafe { CStr::from_ptr(pl).to_str() };
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
pub extern "C" fn encode_coordinates_ffi(coords: Array, precision: uint32_t) -> *mut c_char {
    let s: String = string_from_arr(coords, precision);
    match CString::new(s) {
        Ok(res) => res.into_raw(),
        // It's arguably better to fail noisily, but this is robust
        Err(_) => {
            CString::new("Couldn't decode Polyline".to_string())
                .unwrap()
                .into_raw()
        }
    }
}

/// Free Array memory which Rust has allocated across the FFI boundary
///
/// # Safety
///
/// This function is unsafe because it accesses a raw pointer which could contain arbitrary data
#[no_mangle]
pub extern "C" fn drop_float_array(arr: Array) {
    if arr.data.is_null() {
        return;
    }
    let _: Vec<_> = arr.into();
}

/// Free `CString` memory which Rust has allocated across the FFI boundary
///
/// # Safety
///
/// This function is unsafe because it accesses a raw pointer which could contain arbitrary data
#[no_mangle]
pub extern "C" fn drop_cstring(p: *mut c_char) {
    let _ = unsafe { CString::from_raw(p) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;
    use std::ffi::{CString, CStr};

    #[test]
    fn test_array_conversion() {
        let original = vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]];
        // move into an Array, and leak it
        let arr: Array = original.into();
        // move back into a Vec -- leaked value still needs to be dropped
        let converted: Vec<_> = arr.into();
        assert_eq!(&converted, &[[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]]);
        // drop it
        drop_float_array(converted.into());
    }

    #[test]
    fn test_drop_empty_float_array() {
        let original = vec![[1.0, 2.0], [3.0, 4.0]];
        // move into an Array, and leak it
        let mut arr: Array = original.into();
        // zero Array contents
        arr.data = ptr::null();
        drop_float_array(arr);
    }

    #[test]
    fn test_coordinate_conversion() {
        let input = vec![[1.0, 2.0], [3.0, 4.0]];
        let output = "_ibE_seK_seK_seK";
        let input_arr: Array = input.into();
        let transformed: String = super::string_from_arr(input_arr, 5);
        assert_eq!(transformed, output);
    }

    #[test]
    fn test_string_conversion() {
        let input = "_ibE_seK_seK_seK";
        let output = [[1.0, 2.0], [3.0, 4.0]];
        // String to Array
        let transformed: Array = super::arr_from_string(input, 5);
        // Array to Vec
        let transformed_arr: Vec<_> = transformed.into();
        assert_eq!(&transformed_arr, &output);
    }

    #[test]
    #[should_panic]
    fn test_bad_string_conversion() {
        let input = "_p~iF~ps|U_u🗑lLnnqC_mqNvxq`@";
        let output = vec![[1.0, 2.0], [3.0, 4.0]];
        // String to Array
        let transformed: Array = super::arr_from_string(input, 5);
        // Array to Vec
        let transformed_arr: Vec<_> = transformed.into();
        // this will fail, bc transformed_arr is [[NaN, NaN]]
        assert_eq!(transformed_arr, output.as_slice());
    }

    #[test]
    fn test_ffi_polyline_decoding() {
        let result: Vec<_> =
            decode_polyline_ffi(CString::new("_ibE_seK_seK_seK").unwrap().as_ptr(), 5).into();
        assert_eq!(&result, &[[1.0, 2.0], [3.0, 4.0]]);
        drop_float_array(result.into());
    }

    #[test]
    #[should_panic]
    fn test_bad_precision_decode() {
        let result: Vec<_> =
            decode_polyline_ffi(CString::new("_ibE_seK_seK_seK").unwrap().as_ptr(), 7).into();
        assert_eq!(&result, &[[1.0, 2.0], [3.0, 4.0]]);
        drop_float_array(result.into());
    }

    #[test]
    fn test_ffi_coordinate_encoding() {
        let input: Array = vec![[1.0, 2.0], [3.0, 4.0]].into();
        let output = "_ibE_seK_seK_seK".to_string();
        let pl = encode_coordinates_ffi(input, 5);
        // Allocate a new String
        let result = unsafe { CStr::from_ptr(pl).to_str().unwrap() };
        assert_eq!(&result, &output);
        // Drop received FFI data
        drop_cstring(pl);
    }

    #[test]
    #[should_panic]
    fn test_bad_precision_encode() {
        let input: Array = vec![[1.0, 2.0], [3.0, 4.0]].into();
        let output = "_ibE_seK_seK_seK".to_string();
        let pl = encode_coordinates_ffi(input, 4);
        // Allocate a new String
        let result = unsafe { CStr::from_ptr(pl).to_str().unwrap() };
        assert_eq!(&result, &output);
        // Drop received FFI data
        drop_cstring(pl);
    }
    #[test]
    fn test_long_vec() {
        let arr: Array = include!("../test_fixtures/berlin.rs").into();
        let s = include!("../test_fixtures/berlin_decoded.rs");
        for _ in 0..9999 {
            let a = arr.clone();
            let s_ = s.clone();
            let n = 5;
            let encoded = encode_coordinates_ffi(a, n);
            let result = unsafe { CStr::from_ptr(encoded).to_str().unwrap() };
            assert_eq!(&result, &s_);
            drop_cstring(encoded);
        }
    }
}

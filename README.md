[![Build Status](https://travis-ci.org/urschrei/polyline-ffi.svg?branch=master)](https://travis-ci.org/urschrei/polyline-ffi) [![Build status](https://ci.appveyor.com/api/projects/status/b7706682i39l1on9/branch/master?svg=true)](https://ci.appveyor.com/project/urschrei/polyline-ffi/branch/master) [![Coverage Status](https://coveralls.io/repos/github/urschrei/polyline-ffi/badge.svg?branch=master)](https://coveralls.io/github/urschrei/polyline-ffi?branch=master)

# FFI Bindings for the [rust-polyline](https://github.com/georust/rust-polyline) Crate


## `decode_polyline_ffi`
Convert a Polyline into an array of coordinates.  
Callers must pass two arguments:

- a pointer to a `NUL`-terminated character array (`char*`)
- an unsigned 32-bit `int` for precision (`5` for Google Polylines, `6` for OSRM and Valhalla Polylines)
Returns an `Array` struct with two fields:
- `len`, an integer of type `size_t`, denoting the array length
- `data`, a void pointer to a nested double-precision float array: `[[1.0, 2.0], [3.0, 4.0]]`.

## `drop_float_array`
Free memory pointed to by `Array`, which Rust has allocated across the FFI boundary.  
Callers must pass the same `Array` struct that was received from `decode_polyline_ffi`.

## `encode_coordinates_ffi`
Convert coordinates into a Polyline
Callers must pass a struct, with two members:
- `len`, the array length
- `data`, a void pointer to a nested double-precision float array: `[[1.0, 2.0], [3.0, 4.0]]`  
The return type is a pointer to a C character array (`char*`).

## `drop_cstring`
Free memory pointed to by `char*`, which Rust has allocated across the FFI boundary.  
Callers must pass the same `char*` they receive from `encode_coordinates_ffi`.

# Binaries
Compressed binaries are available for Linux (64-bit), OSX (64-bit), and Windows (32-bit and 64-bit), from the [releases](https://github.com/urschrei/polyline-ffi/releases) page.  
The Linux binary has been built using the manylinux1 (CentOS 5.11) Docker image, and is widely compatible.  
Both Linux and OSX binaries have been built with `rpath` support.

# License
[MIT](license.txt)

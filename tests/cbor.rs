#![cfg(feature = "std")]
#![cfg(feature = "cbor")]
#![cfg(not(target_arch = "wasm32"))]

use cddl::{
  cddl_from_str,
  validator::{cbor::CBORValidator, cbor_value, validate_cbor_from_slice, Validator},
};
use ciborium::value::Value;
use indoc::indoc;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::error::Error;

#[rustfmt::skip] 
pub mod cbor {
    // example values from rfc7049 appendix A
    pub const BOOL_FALSE:   &[u8] = b"\xF4";
    pub const BOOL_TRUE:    &[u8] = b"\xF5";
    pub const NULL:         &[u8] = b"\xF6";
    pub const UNDEFINED:    &[u8] = b"\xF7";

    pub const INT_0:        &[u8] = b"\x00";
    pub const INT_1:        &[u8] = b"\x01";
    pub const INT_23:       &[u8] = b"\x17";
    pub const INT_24:       &[u8] = b"\x18\x18";
    pub const NINT_1000:    &[u8] = b"\x39\x03\xe7";  // -1000

    pub const FLOAT_0_0:    &[u8] = b"\xf9\x00\x00";            // #7.25 (f16)
    pub const FLOAT_1_0:    &[u8] = b"\xf9\x3c\x00";            // #7.25 (f16)
    pub const FLOAT_1E5:    &[u8] = b"\xfa\x47\xc3\x50\x00";    // #7.26 (f32)
    pub const FLOAT_1E300:  &[u8] = b"\xfb\x7e\x37\xe4\x3c\x88\x00\x75\x9c"; // #7.27 (f64)

    pub const ARRAY_EMPTY:  &[u8] = b"\x80";              // []
    pub const ARRAY_123:    &[u8] = b"\x83\x01\x02\x03";  // [1,2,3]
    pub const ARRAY_1_23_45:&[u8] = b"\x83\x01\x82\x02\x03\x82\x04\x05";  // [1, [2, 3], [4, 5]]

    pub const TEXT_EMPTY:   &[u8] = b"\x60";
    pub const TEXT_IETF:    &[u8] = b"\x64\x49\x45\x54\x46";
    pub const TEXT_CJK:     &[u8] = b"\x63\xe6\xb0\xb4";    // "水

    pub const BYTES_EMPTY:  &[u8] = b"\x40";
    pub const BYTES_1234:   &[u8] = b"\x44\x01\x02\x03\x04"; // hex 01020304

    // Simple values (major type 7)
    pub const SIMPLE_0:     &[u8] = b"\xe0";        // simple(0) - unassigned
    pub const SIMPLE_19:    &[u8] = b"\xf3";        // simple(19) - unassigned
    pub const SIMPLE_32:    &[u8] = b"\xf8\x20";    // simple(32) - unassigned (two-byte encoding)
    pub const SIMPLE_255:   &[u8] = b"\xf8\xff";    // simple(255) - unassigned (two-byte encoding)
}

// These data structures exist so that we can serialize some more complex
// beyond the RFC examples.
#[derive(Debug, Serialize, Deserialize)]
struct PersonStruct {
  name: String,
  age: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersonTuple(String, u32);

#[derive(Debug, Serialize, Deserialize)]
struct BackwardsTuple(u32, String);

#[derive(Debug, Serialize, Deserialize)]
struct LongTuple(String, u32, u32);

#[derive(Debug, Serialize, Deserialize)]
struct ShortTuple(String);

#[derive(Debug, Serialize, Deserialize)]
struct KitchenSink(String, u32, f64, bool);

#[test]
fn validate_cbor_bool() {
  let cddl_input = r#"thing = true"#;
  validate_cbor_from_slice(cddl_input, cbor::BOOL_TRUE, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::BOOL_FALSE, None).unwrap_err();
  validate_cbor_from_slice(cddl_input, cbor::NULL, None).unwrap_err();
}

#[test]
fn validate_cbor_float() {
  let cddl_input = r#"thing = 0.0"#;
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_0_0, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_1_0, None).unwrap_err();

  let cddl_input = r#"thing = float"#;
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_1_0, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_1E5, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_1E300, None).unwrap();

  let cddl_input = r#"thing = float16"#;
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_1_0, None).unwrap();

  // "Too small" floats should not cause a validation error.
  // "Canonical CBOR" suggests that floats should be shrunk to the smallest
  // size that can represent the value.  So 1.0 can be stored in 16 bits,
  // even if the CDDL specifies float64.
  let cddl_input = r#"thing = float32"#;
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_1_0, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_1E5, None).unwrap();

  let cddl_input = r#"thing = float64"#;
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_1_0, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_1E300, None).unwrap();

  // TODO: check that large floats don't validate against a smaller size.
  // E.g. CBOR #7.27 (64-bit) shouldn't validate against "float16" or "float32".
}

#[test]
fn validate_cbor_integer() {
  let cddl_input = r#"thing = 23 / 24"#;
  validate_cbor_from_slice(cddl_input, cbor::INT_23, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::INT_24, None).unwrap();
  let cddl_input = r#"thing = 1"#;
  validate_cbor_from_slice(cddl_input, cbor::NULL, None).unwrap_err();
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_1_0, None).unwrap_err();
  validate_cbor_from_slice(cddl_input, cbor::BOOL_TRUE, None).unwrap_err();
  let cddl_input = r#"thing = int"#;
  validate_cbor_from_slice(cddl_input, cbor::INT_0, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::INT_24, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::NINT_1000, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_1_0, None).unwrap_err();
  let cddl_input = r#"thing = uint"#;
  validate_cbor_from_slice(cddl_input, cbor::INT_0, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::INT_24, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::NINT_1000, None).unwrap_err();
}

#[test]
fn validate_cbor_textstring() {
  let cddl_input = r#"thing = tstr"#;
  validate_cbor_from_slice(cddl_input, cbor::TEXT_EMPTY, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::TEXT_IETF, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::TEXT_CJK, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::BYTES_EMPTY, None).unwrap_err();
}

#[test]
fn validate_cbor_bytestring() {
  let cddl_input = r#"thing = bstr"#;
  validate_cbor_from_slice(cddl_input, cbor::BYTES_EMPTY, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::BYTES_1234, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::TEXT_EMPTY, None).unwrap_err();
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_123, None).unwrap_err();
}

#[test]
fn validate_cbor_array() {
  let cddl_input = r#"thing = []"#;
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_EMPTY, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::NULL, None).unwrap_err();

  validate_cbor_from_slice(cddl_input, cbor::ARRAY_123, None).unwrap_err();

  let cddl_input = r#"thing = [1, 2, 3]"#;
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_123, None).unwrap();
}

#[test]
fn validate_cbor_group() {
  let cddl_input = r#"thing = (* int)"#;
  validate_cbor_from_slice(cddl_input, cbor::INT_0, None).unwrap();
}

#[test]
fn validate_cbor_homogenous_array() {
  let cddl_input = r#"thing = [* int]"#; // zero or more
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_EMPTY, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_123, None).unwrap();
  let cddl_input = r#"thing = [+ int]"#; // one or more
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_123, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_EMPTY, None).unwrap_err();
  let cddl_input = r#"thing = [? int]"#; // zero or one
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_EMPTY, None).unwrap();
  let mut cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&[42], &mut cbor_bytes).unwrap();
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_123, None).unwrap_err();

  let cddl_input = r#"thing = [* tstr]"#;
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_123, None).unwrap_err();

  // Alias type.  Note the rule we want to validate must come first.
  let cddl_input = r#"thing = [* zipcode]  zipcode = int"#;
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_EMPTY, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_123, None).unwrap();
}

#[test]
fn validate_cbor_array_groups() {
  let cddl_input = r#"thing = [int, (int, int)]"#;
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_123, None).unwrap();
  // TODO: try splitting arrays into groups a few other ways:
  // [(int, int, int)]
  // [* (int)]
  // [* (int, int)]
}

#[test]
fn validate_cbor_array_record() {
  let cddl_input = r#"thing = [a: int, b: int, c: int]"#;
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_123, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_EMPTY, None).unwrap_err();

  let cddl_input = r#"thing = [a: tstr, b: int]"#;

  let input = PersonTuple("Alice".to_string(), 42);
  let mut cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&input, &mut cbor_bytes).unwrap();
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap();

  let input = BackwardsTuple(43, "Carol".to_string());
  let mut cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&input, &mut cbor_bytes).unwrap();
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap_err();

  let input = LongTuple("David".to_string(), 44, 45);
  let mut cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&input, &mut cbor_bytes).unwrap();
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap_err();

  let input = ShortTuple("Eve".to_string());
  let mut cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&input, &mut cbor_bytes).unwrap();
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap_err();

  let cddl_input = r#"thing = [a: tstr, b: uint, c: float32, d: bool]"#;

  let input = KitchenSink("xyz".to_string(), 17, 9.9, false);
  let mut cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&input, &mut cbor_bytes).unwrap();
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap();

  // FIXME: there isn't any way at present to serialize a struct
  // into a CBOR array. See https://github.com/pyfisch/cbor/issues/107
  // let input = PersonStruct{name: "Bob".to_string(), age: 43};
  // let mut cbor_bytes = Vec::new();
  // ciborium::ser::into_writer(&input, &mut cbor_bytes).unwrap();
  // validate_cbor_from_slice(cddl_input, &cbor_bytes).unwrap();

  validate_cbor_from_slice(cddl_input, cbor::ARRAY_123, None).unwrap_err();
}

#[test]
fn validate_cbor_map() {
  let input = PersonStruct {
    name: "Bob".to_string(),
    age: 43,
  };
  let mut cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&input, &mut cbor_bytes).unwrap();
  let cddl_input = r#"thing = {name: tstr, age: int}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap();
  let cddl_input = r#"thing = {name: tstr, ? age: int}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap();

  // Ensure that keys are optional if the occurrence is "?" or "*"
  // and required if the occurrence is "+"
  let cddl_input = r#"thing = {name: tstr, age: int, ? minor: bool}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap();
  let cddl_input = r#"thing = {name: tstr, age: int, * minor: bool}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap();
  let cddl_input = r#"thing = {name: tstr, age: int, + minor: bool}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap_err();

  let cddl_input = r#"thing = {name: tstr, age: tstr}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap_err();

  let cddl_input = r#"thing = {name: tstr}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap_err();

  // "* keytype => valuetype" is the expected syntax for collecting
  // any remaining key/value pairs of the expected type.
  let cddl_input = r#"thing = {* tstr => any}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap();
  let cddl_input = r#"thing = {name: tstr, * tstr => any}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap();
  let cddl_input = r#"thing = {name: tstr, age: int, * tstr => any}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap();
  let cddl_input = r#"thing = {+ tstr => any}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap();

  // Should fail because the CBOR input has one entry that can't be
  // collected because the value type doesn't match.
  let cddl_input = r#"thing = {* tstr => int}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap_err();

  // Should fail because the CBOR input has two entries that can't be
  // collected because the key type doesn't match.
  let cddl_input = r#"thing = {* int => any}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap_err();

  let cddl_input = r#"thing = {name: tstr, age: int, minor: bool}"#;
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap_err();

  let cddl_input = r#"thing = {x: int, y: int, z: int}"#;
  validate_cbor_from_slice(cddl_input, cbor::ARRAY_123, None).unwrap_err();
}

#[test]
fn verify_large_tag_values() -> Result<(), Box<dyn Error>> {
  let input = r#"
        thing = #6.8386104246373017956(tstr) / #6.42(tstr)
    "#;

  // Test tag 42 (small tag value)
  let test_str = "test";
  let cbor = Value::Tag(42, Box::new(Value::Text(test_str.to_string())));
  let mut bytes = Vec::new();
  ciborium::ser::into_writer(&cbor, &mut bytes)?;
  assert!(validate_cbor_from_slice(input, &bytes, None).is_ok());

  // Test tag 8386104246373017956 (large tag value)
  let cbor = Value::Tag(
    8386104246373017956,
    Box::new(Value::Text(test_str.to_string())),
  );
  let mut bytes = Vec::new();
  ciborium::ser::into_writer(&cbor, &mut bytes)?;
  assert!(validate_cbor_from_slice(input, &bytes, None).is_ok());

  // Test wrong tag value - should fail
  let cbor = Value::Tag(99, Box::new(Value::Text(test_str.to_string())));
  let mut bytes = Vec::new();
  ciborium::ser::into_writer(&cbor, &mut bytes)?;
  assert!(validate_cbor_from_slice(input, &bytes, None).is_err());

  Ok(())
}

#[test]
fn validate_range_operators() -> Result<(), Box<dyn Error>> {
  let cddl = indoc!(
    r#"
        test = {
            inclusive: 5..10,      ; inclusive-inclusive range 
            exclusive: 5...10,     ; inclusive-exclusive range (per RFC 8610)
        }
        "#
  );

  let cddl = cddl_from_str(cddl, true)?;

  // Test inclusive range (..) and inclusive-exclusive range (...)
  let test = Value::Map(vec![
    (
      Value::Text("inclusive".to_string()),
      Value::Integer(5.into()),
    ),
    (
      Value::Text("exclusive".to_string()),
      Value::Integer(5.into()),
    ),
  ]);
  let test: cbor_value::Value = test.into();
  let mut cv = CBORValidator::new(&cddl, test, None);
  cv.validate()?;

  let test = Value::Map(vec![
    (
      Value::Text("inclusive".to_string()),
      Value::Integer(10.into()),
    ),
    (
      Value::Text("exclusive".to_string()),
      Value::Integer(9.into()),
    ),
  ]);
  let test: cbor_value::Value = test.into();
  let mut cv = CBORValidator::new(&cddl, test, None);
  cv.validate()?;

  // Test fail cases
  let test = Value::Map(vec![
    (
      Value::Text("inclusive".to_string()),
      Value::Integer(10.into()),
    ),
    (
      Value::Text("exclusive".to_string()),
      Value::Integer(10.into()),
    ), // Should fail - 10 is exclusive
  ]);
  let test: cbor_value::Value = test.into();
  let mut cv = CBORValidator::new(&cddl, test, None);
  assert!(
    cv.validate().is_err(),
    "10 should fail inclusive-exclusive range 5...10"
  );

  let test = Value::Map(vec![
    (
      Value::Text("inclusive".to_string()),
      Value::Integer(4.into()),
    ), // Should fail - 4 is out of range
    (
      Value::Text("exclusive".to_string()),
      Value::Integer(5.into()),
    ),
  ]);
  let test: cbor_value::Value = test.into();
  let mut cv = CBORValidator::new(&cddl, test, None);
  assert!(
    cv.validate().is_err(),
    "4 should fail inclusive range 5..10"
  );

  Ok(())
}

#[test]
fn validate_cbor_size_range_with_constant() -> Result<(), Box<dyn Error>> {
  let cddl_input = r#"
        person = {name: tstr .size (1..max_tstr_length), age: uint}
        max_tstr_length = 100
    "#;

  // --- Positive Test (name length within range) ---
  let valid_person = Value::Map(vec![
    (
      Value::Text("name".to_string()),
      Value::Text("Alice".to_string()),
    ), // Length 5
    (Value::Text("age".to_string()), Value::Integer(30.into())),
  ]);
  let mut valid_cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&valid_person, &mut valid_cbor_bytes)?;
  validate_cbor_from_slice(cddl_input, &valid_cbor_bytes, None)
    .expect("Validation should succeed for name length within range");

  // --- Positive Test (name length at max boundary) ---
  let max_len_name = "a".repeat(100);
  let valid_person_max = Value::Map(vec![
    (Value::Text("name".to_string()), Value::Text(max_len_name)), // Length 100
    (Value::Text("age".to_string()), Value::Integer(30.into())),
  ]);
  let mut valid_max_cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&valid_person_max, &mut valid_max_cbor_bytes)?;
  validate_cbor_from_slice(cddl_input, &valid_max_cbor_bytes, None)
    .expect("Validation should succeed for name length at max boundary");

  // --- Negative Test (name length exceeds range) ---
  let long_name = "a".repeat(101);
  let invalid_person_long = Value::Map(vec![
    (Value::Text("name".to_string()), Value::Text(long_name)), // Length 101
    (Value::Text("age".to_string()), Value::Integer(30.into())),
  ]);
  let mut invalid_long_cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&invalid_person_long, &mut invalid_long_cbor_bytes)?;
  validate_cbor_from_slice(cddl_input, &invalid_long_cbor_bytes, None)
    .expect_err("Validation should fail for name length exceeding range");

  // --- Negative Test (name length below range - zero length) ---
  let empty_name = "";
  let invalid_person_empty = Value::Map(vec![
    (
      Value::Text("name".to_string()),
      Value::Text(empty_name.to_string()),
    ), // Length 0
    (Value::Text("age".to_string()), Value::Integer(30.into())),
  ]);
  let mut invalid_empty_cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&invalid_person_empty, &mut invalid_empty_cbor_bytes)?;
  validate_cbor_from_slice(cddl_input, &invalid_empty_cbor_bytes, None)
    .expect_err("Validation should fail for zero-length name");

  Ok(())
}

/// Test for GitHub issue #90: CBOR validation fails for non-standard simple values.
/// CDDL `#7.N` should match CBOR simple value N for unassigned simple values (0-19, 32-255).
#[test]
fn validate_cbor_simple_values() {
  // Simple value 32 (unassigned, two-byte encoded as 0xf8 0x20)
  let cddl_input = r#"thing = #7.32"#;
  validate_cbor_from_slice(cddl_input, cbor::SIMPLE_32, None).unwrap();

  // Wrong simple value should fail
  validate_cbor_from_slice(cddl_input, cbor::SIMPLE_255, None).unwrap_err();

  // Simple value 0 (one-byte encoded)
  let cddl_input = r#"thing = #7.0"#;
  validate_cbor_from_slice(cddl_input, cbor::SIMPLE_0, None).unwrap();

  // Simple value 19 (one-byte encoded)
  let cddl_input = r#"thing = #7.19"#;
  validate_cbor_from_slice(cddl_input, cbor::SIMPLE_19, None).unwrap();

  // Simple value 255 (two-byte encoded)
  let cddl_input = r#"thing = #7.255"#;
  validate_cbor_from_slice(cddl_input, cbor::SIMPLE_255, None).unwrap();

  // Major type 7 without constraint should match any simple value
  let cddl_input = r#"thing = #7"#;
  validate_cbor_from_slice(cddl_input, cbor::SIMPLE_0, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::SIMPLE_32, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::SIMPLE_255, None).unwrap();

  // #7 should also match booleans, null, and floats
  validate_cbor_from_slice(cddl_input, cbor::BOOL_TRUE, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::BOOL_FALSE, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::NULL, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::FLOAT_1_0, None).unwrap();

  // Standard simple values via #7.N
  let cddl_input = r#"thing = #7.20"#;
  validate_cbor_from_slice(cddl_input, cbor::BOOL_FALSE, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::BOOL_TRUE, None).unwrap_err();

  let cddl_input = r#"thing = #7.21"#;
  validate_cbor_from_slice(cddl_input, cbor::BOOL_TRUE, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::BOOL_FALSE, None).unwrap_err();

  let cddl_input = r#"thing = #7.22"#;
  validate_cbor_from_slice(cddl_input, cbor::NULL, None).unwrap();
  validate_cbor_from_slice(cddl_input, cbor::BOOL_TRUE, None).unwrap_err();
}

/// Regression test for https://github.com/anweiss/cddl/issues/465
#[test]
fn validate_cbor_array_record_extra_elements() {
  let cddl_input = r#"thing = [a: tstr, b: int]"#;

  // Exact match: ["testString", 1] should pass
  let input = PersonTuple("testString".to_string(), 1);
  let mut cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&input, &mut cbor_bytes).unwrap();
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap();

  // Extra element: ["testString", 1, 2] must fail
  let input = LongTuple("testString".to_string(), 1, 2);
  let mut cbor_bytes = Vec::new();
  ciborium::ser::into_writer(&input, &mut cbor_bytes).unwrap();
  validate_cbor_from_slice(cddl_input, &cbor_bytes, None).unwrap_err();
}

#[test]
fn validate_decfrac_and_bigfloat() -> Result<(), Box<dyn Error>> {
  // Helper to encode a ciborium Value to CBOR bytes
  fn cbor_encode(val: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(val, &mut bytes).unwrap();
    bytes
  }

  // Valid decfrac: Tag(4, [int, integer]) e.g. 273.15 = 27315 * 10^(-2)
  let cddl_input = r#"temperature = decfrac"#;
  let cbor_val = Value::Tag(
    4,
    Box::new(Value::Array(vec![
      Value::Integer((-2).into()),
      Value::Integer(27315.into()),
    ])),
  );
  let bytes = cbor_encode(&cbor_val);
  validate_cbor_from_slice(cddl_input, &bytes, None)?;

  // Valid bigfloat: Tag(5, [int, integer]) e.g. 1.5 = 3 * 2^(-1)
  let cddl_input = r#"measurement = bigfloat"#;
  let cbor_val = Value::Tag(
    5,
    Box::new(Value::Array(vec![
      Value::Integer((-1).into()),
      Value::Integer(3.into()),
    ])),
  );
  let bytes = cbor_encode(&cbor_val);
  validate_cbor_from_slice(cddl_input, &bytes, None)?;

  // Invalid: wrong tag for decfrac (tag 5 instead of 4)
  let cddl_input = r#"temperature = decfrac"#;
  let cbor_val = Value::Tag(
    5,
    Box::new(Value::Array(vec![
      Value::Integer((-2).into()),
      Value::Integer(27315.into()),
    ])),
  );
  let bytes = cbor_encode(&cbor_val);
  assert!(validate_cbor_from_slice(cddl_input, &bytes, None).is_err());

  // Invalid: wrong tag for bigfloat (tag 4 instead of 5)
  let cddl_input = r#"measurement = bigfloat"#;
  let cbor_val = Value::Tag(
    4,
    Box::new(Value::Array(vec![
      Value::Integer((-1).into()),
      Value::Integer(3.into()),
    ])),
  );
  let bytes = cbor_encode(&cbor_val);
  assert!(validate_cbor_from_slice(cddl_input, &bytes, None).is_err());

  // Invalid: not an array inside tag
  let cddl_input = r#"temperature = decfrac"#;
  let cbor_val = Value::Tag(4, Box::new(Value::Integer(42.into())));
  let bytes = cbor_encode(&cbor_val);
  assert!(validate_cbor_from_slice(cddl_input, &bytes, None).is_err());

  // Invalid: array with wrong types (float instead of int for exponent)
  let cddl_input = r#"temperature = decfrac"#;
  let cbor_val = Value::Tag(
    4,
    Box::new(Value::Array(vec![
      Value::Float(1.5),
      Value::Integer(27315.into()),
    ])),
  );
  let bytes = cbor_encode(&cbor_val);
  assert!(validate_cbor_from_slice(cddl_input, &bytes, None).is_err());

  // Valid: using with explicit tag notation
  let cddl_input = r#"mytype = #6.4([int, integer])"#;
  let cbor_val = Value::Tag(
    4,
    Box::new(Value::Array(vec![
      Value::Integer((-2).into()),
      Value::Integer(27315.into()),
    ])),
  );
  let bytes = cbor_encode(&cbor_val);
  validate_cbor_from_slice(cddl_input, &bytes, None)?;

  // Valid: bigfloat with bignum mantissa (tag 2 biguint)
  let cddl_input = r#"big_measurement = bigfloat"#;
  let cbor_val = Value::Tag(
    5,
    Box::new(Value::Array(vec![
      Value::Integer((-1).into()),
      Value::Tag(2, Box::new(Value::Bytes(vec![0x01, 0x00]))),
    ])),
  );
  let bytes = cbor_encode(&cbor_val);
  validate_cbor_from_slice(cddl_input, &bytes, None)?;

  Ok(())
}

fn cbor_encode(val: &Value) -> Vec<u8> {
  let mut bytes = Vec::new();
  ciborium::ser::into_writer(val, &mut bytes).unwrap();
  bytes
}

// Regression: nested map under `* k => {+ k2 => v}` must validate
// (RFC 8610 §3.5).
#[test]
fn validate_nested_map_member_value() {
  let cddl_input = r#"start = {* bytes => {+ bytes => uint}}"#;

  let valid = Value::Map(vec![(
    Value::Bytes(vec![0x61, 0x62]),
    Value::Map(vec![(Value::Bytes(vec![0x63]), Value::Integer(5.into()))]),
  )]);
  let bytes = cbor_encode(&valid);
  validate_cbor_from_slice(cddl_input, &bytes, None).unwrap();

  // Inner value must be a map, not a uint.
  let invalid = Value::Map(vec![(
    Value::Bytes(vec![0x61, 0x62]),
    Value::Integer(5.into()),
  )]);
  let bytes = cbor_encode(&invalid);
  assert!(validate_cbor_from_slice(cddl_input, &bytes, None).is_err());
}

// Regression: a typename with generic args used as a map type
// (`outer<inner_value>`) must resolve to its body, not get compared whole
// against `bstr` via the keyed-map's outer key type.
#[test]
fn validate_generic_typename_as_map() {
  let cddl_input = r#"
    start = outer<positive>
    outer<v> = {* outer_key => {+ inner_key => v}}
    outer_key = bytes .size 4
    inner_key = bytes .size (0 .. 4)
    positive = 1 .. 18446744073709551615
  "#;

  // Valid: 4-byte outer key, 3-byte inner key, value 1.
  let valid = Value::Map(vec![(
    Value::Bytes(vec![0x01, 0x02, 0x03, 0x04]),
    Value::Map(vec![(
      Value::Bytes(vec![0x66, 0x6f, 0x6f]),
      Value::Integer(1.into()),
    )]),
  )]);
  let bytes = cbor_encode(&valid);
  validate_cbor_from_slice(cddl_input, &bytes, None).unwrap();

  // Invalid: value 0 violates positive's range.
  let invalid_zero = Value::Map(vec![(
    Value::Bytes(vec![0x01, 0x02, 0x03, 0x04]),
    Value::Map(vec![(
      Value::Bytes(vec![0x66, 0x6f, 0x6f]),
      Value::Integer(0.into()),
    )]),
  )]);
  let bytes = cbor_encode(&invalid_zero);
  assert!(validate_cbor_from_slice(cddl_input, &bytes, None).is_err());
}

// Regression: array entries past index 0 must still be validated when the
// type is `[primitive, generic_typename<arg>]`.
#[test]
fn validate_array_with_generic_typename_entry() {
  let cddl_input = r#"
    start = [counter, outer<positive>]
    counter = uint
    outer<v> = {* outer_key => {+ inner_key => v}}
    outer_key = bytes .size 4
    inner_key = bytes .size (0 .. 4)
    positive = 1 .. 18446744073709551615
  "#;

  // Valid: [counter, outer-map].
  let valid_pair = Value::Array(vec![
    Value::Integer(1_000_000.into()),
    Value::Map(vec![(
      Value::Bytes(vec![0x01, 0x02, 0x03, 0x04]),
      Value::Map(vec![(
        Value::Bytes(vec![0x66, 0x6f, 0x6f]),
        Value::Integer(1.into()),
      )]),
    )]),
  ]);
  let bytes = cbor_encode(&valid_pair);
  validate_cbor_from_slice(cddl_input, &bytes, None).unwrap();

  // Invalid: second element is a string instead of a map.
  let invalid_garbage = Value::Array(vec![
    Value::Integer(1.into()),
    Value::Text("wrong".to_string()),
  ]);
  let bytes = cbor_encode(&invalid_garbage);
  assert!(validate_cbor_from_slice(cddl_input, &bytes, None).is_err());

  // Invalid: second element is a map with the wrong inner shape (uint, not nested map).
  let invalid_inner = Value::Array(vec![
    Value::Integer(1.into()),
    Value::Map(vec![(
      Value::Bytes(vec![0x01, 0x02, 0x03, 0x04]),
      Value::Integer(5.into()),
    )]),
  ]);
  let bytes = cbor_encode(&invalid_inner);
  assert!(validate_cbor_from_slice(cddl_input, &bytes, None).is_err());

  // Invalid: zero leaf value violates positive.
  let invalid_zero = Value::Array(vec![
    Value::Integer(1.into()),
    Value::Map(vec![(
      Value::Bytes(vec![0x01, 0x02, 0x03, 0x04]),
      Value::Map(vec![(
        Value::Bytes(vec![0x66, 0x6f, 0x6f]),
        Value::Integer(0.into()),
      )]),
    )]),
  ]);
  let bytes = cbor_encode(&invalid_zero);
  assert!(validate_cbor_from_slice(cddl_input, &bytes, None).is_err());
}

// Regression: negative integer literals down to -2^64 (the CBOR nint floor)
// must parse and round-trip through the AST. Requires IntValue: i128, not i64.
#[test]
fn validate_negative_below_i64_min() {
  // -2^64 is the smallest CBOR int (major type 1, payload 0xff_ff_ff_ff_ff_ff_ff_ff).
  // It does NOT fit in i64 (i64::MIN = -2^63).
  let cddl_input = r#"start = -18446744073709551616"#;

  let v_min = Value::Integer(ciborium::value::Integer::try_from(-1_i128 << 64).unwrap());
  validate_cbor_from_slice(cddl_input, &cbor_encode(&v_min), None).unwrap();

  // Wrong value should fail.
  let v_neg_one = Value::Integer((-1_i128).try_into().unwrap());
  assert!(validate_cbor_from_slice(cddl_input, &cbor_encode(&v_neg_one), None).is_err());
}

// Regression: an array whose group definition has `?` (Optional) entries
// must accept every length in `mandatory..=mandatory+optional`, not only the
// fixed maximum. `entry_counts_from_group` previously only inspected the
// occur of the second entry (idx==1), so optionals at any other position
// produced a single fixed count and shorter inputs were rejected.
#[test]
fn optional_entry_length_accepts_short_arrays() {
  let cddl = r#"start = [a: int, b: tstr, ? c: bytes]"#;

  // Length 2 (optional absent) must pass.
  let v = Value::Array(vec![Value::Integer(1.into()), Value::Text("ok".into())]);
  validate_cbor_from_slice(cddl, &cbor_encode(&v), None).unwrap();

  // Length 3 (optional present) must pass.
  let v = Value::Array(vec![
    Value::Integer(1.into()),
    Value::Text("ok".into()),
    Value::Bytes(vec![0xde, 0xad]),
  ]);
  validate_cbor_from_slice(cddl, &cbor_encode(&v), None).unwrap();

  // Length 1 must fail.
  let v = Value::Array(vec![Value::Integer(1.into())]);
  assert!(validate_cbor_from_slice(cddl, &cbor_encode(&v), None).is_err());

  // Length 4 must fail.
  let v = Value::Array(vec![
    Value::Integer(1.into()),
    Value::Text("ok".into()),
    Value::Bytes(vec![0xde, 0xad]),
    Value::Integer(99.into()),
  ]);
  assert!(validate_cbor_from_slice(cddl, &cbor_encode(&v), None).is_err());
}

// Regression: bareword keys inside an array (e.g. `[a: int, b: tstr]`) are
// pure documentation per RFC 8610 §3.5.2. When the bareword identifier
// happens to also name a rule whose body is an array (e.g. `pair` =
// `[fst, snd]`), the validator must NOT short-circuit by re-running that
// rule's body against the surrounding array. It used to, which made
// `[a, b, c, pair]` reject every record because the validator compared the
// 4-element record against the 2-element pair.
#[test]
fn array_member_key_bareword_is_documentation_only() {
  let cddl = r#"
    start = [+ record]
    record = [tag: tag_kind, idx: uint .size 4, payload: payload_t, pair: pair]
    tag_kind = 0 / 1 / 2 / 3 / 4
    payload_t = #6.121([])
    pair = [fst: uint, snd: uint]
  "#;

  let valid = Value::Array(vec![Value::Array(vec![
    Value::Integer(1.into()),
    Value::Integer(0.into()),
    Value::Tag(121, Box::new(Value::Array(vec![]))),
    Value::Array(vec![Value::Integer(7.into()), Value::Integer(11.into())]),
  ])]);
  validate_cbor_from_slice(cddl, &cbor_encode(&valid), None).unwrap();

  // Real failure should still be reported with a deep path: corrupt the
  // inner pair (1 element instead of 2).
  let invalid = Value::Array(vec![Value::Array(vec![
    Value::Integer(1.into()),
    Value::Integer(0.into()),
    Value::Tag(121, Box::new(Value::Array(vec![]))),
    Value::Array(vec![Value::Integer(7.into())]),
  ])]);
  let err = validate_cbor_from_slice(cddl, &cbor_encode(&invalid), None).expect_err("must fail");
  let msg = err.to_string();
  assert!(
    msg.contains("/0/3"),
    "expected deep path /0/3 in error, got:\n{}",
    msg
  );
}

// Regression: `[+ T]` / `[* T]` over a referenced rule must validate EVERY
// element of the cbor array, not just `cbor[group_entry_idx]`. The earlier
// "nested array in literal position" fast-path inside `visit_type` together
// with the multi-type-choice valid_array_items handling caused later
// elements to be silently accepted.
#[test]
fn homogeneous_array_iterates_every_element() {
  let cddl = r#"
    start = nonempty_set<inner>
    nonempty_set<a> = #6.258([+ a]) / [+ a]
    inner = [int, tstr]
  "#;

  let inner_good = Value::Array(vec![Value::Integer(1.into()), Value::Text("ok".into())]);
  let inner_bad = Value::Array(vec![Value::Integer(2.into()), Value::Integer(99.into())]);

  // Both good — must pass.
  let v = Value::Array(vec![inner_good.clone(), inner_good.clone()]);
  validate_cbor_from_slice(cddl, &cbor_encode(&v), None).unwrap();

  // Good then bad — must fail with deep path /1/1.
  let v = Value::Array(vec![inner_good.clone(), inner_bad.clone()]);
  let err =
    validate_cbor_from_slice(cddl, &cbor_encode(&v), None).expect_err("expected validation error");
  let msg = err.to_string();
  assert!(
    msg.contains("/1/1"),
    "expected deep path /1/1 in failure message, got:\n{}",
    msg
  );

  // Plain [+ int] homogeneous on a primitive must also reject mid-array
  // mismatches.
  let cddl_simple = r#"start = [+ int]"#;
  let v = Value::Array(vec![
    Value::Integer(1.into()),
    Value::Text("oops".into()),
    Value::Integer(3.into()),
  ]);
  let err = validate_cbor_from_slice(cddl_simple, &cbor_encode(&v), None)
    .expect_err("expected validation error");
  assert!(err.to_string().contains("/1"));
}

// Regression: when ALL type-choice alternatives fail, the reported errors
// must carry the deep cbor_location of the failing place inside each choice
// (and the path must show map keys, not raw value debug output).
#[test]
fn type_choice_failure_reports_deep_path() {
  let cddl = r#"
    start = uint / outer<int>
    outer<v> = {* bytes => {+ bytes => v}}
  "#;

  let v = Value::Map(vec![(
    Value::Bytes(vec![0xab]),
    Value::Map(vec![(
      Value::Bytes(vec![0xcd]),
      Value::Text("not-int".into()),
    )]),
  )]);
  let bytes = cbor_encode(&v);
  let err = validate_cbor_from_slice(cddl, &bytes, None).expect_err("expected validation failure");
  let msg = err.to_string();

  // The deep failure path must be present, and it must use the key, not a
  // dump of the surrounding map's debug repr.
  assert!(
    msg.contains("/h'ab'"),
    "expected path to walk into the bytes-keyed entry, got:\n{}",
    msg
  );
  assert!(
    !msg.contains("/Map(["),
    "expected no `Map([...])` debug-dump segment in cbor_location, got:\n{}",
    msg
  );
}

// Regression: `.cborseq` must (a) parse — earlier the grammar listed `cbor`
// before `cborseq` so the shorter prefix `cbor` ate the leading bytes — and
// (b) decode the embedded byte string as a CBOR sequence (RFC 8742 — multiple
// concatenated top-level data items) into an array, not just the first item.
#[test]
fn validate_cborseq_decodes_concatenated_items() {
  fn enc(v: &Value) -> Vec<u8> {
    let mut b = Vec::new();
    ciborium::ser::into_writer(v, &mut b).unwrap();
    b
  }

  // Build a bytestring with three concatenated CBOR ints: 1, 2, 3.
  let mut seq = Vec::new();
  seq.extend_from_slice(&enc(&Value::Integer(1.into())));
  seq.extend_from_slice(&enc(&Value::Integer(2.into())));
  seq.extend_from_slice(&enc(&Value::Integer(3.into())));
  let bstr = Value::Bytes(seq);
  let bytes = enc(&bstr);

  // Fixed-shape match.
  let cddl = r#"start = bstr .cborseq [int, int, int]"#;
  validate_cbor_from_slice(cddl, &bytes, None).unwrap();

  // Homogeneous match.
  let cddl = r#"start = bstr .cborseq [+ int]"#;
  validate_cbor_from_slice(cddl, &bytes, None).unwrap();

  // Empty sequence is also a valid CBOR sequence and should match `[]`.
  let empty = enc(&Value::Bytes(Vec::new()));
  validate_cbor_from_slice(r#"start = bstr .cborseq []"#, &empty, None).unwrap();

  // Wrong inner type: sequence of ints validated against `[+ tstr]` must fail.
  assert!(validate_cbor_from_slice(r#"start = bstr .cborseq [+ tstr]"#, &bytes, None).is_err());
}

// Regression: when a bareword-less array entry is a typename whose body is
// `bstr .size N`, a wrong-size CBOR bytes value at index `i` must report the
// failure with the deep cbor_location `/i`. The control-op-on-array-element
// fast path used to forget to seed the sub-validator's `data_location`, so
// the error fired at the empty top-level path.
#[test]
fn type_choice_size_failure_reports_array_index_path() {
  let cddl = r#"
    start = wrap<inner>
    wrap<a> = #6.258([+ a]) / [+ a]
    inner = [pubkey, signature]
    pubkey = bytes .size 32
    signature = bytes .size 64
  "#;

  // Witness with too-short signature at $[0][1].
  let v = Value::Array(vec![Value::Array(vec![
    Value::Bytes(vec![0x01; 32]),
    Value::Bytes(vec![0x02; 10]),
  ])]);
  let bytes = cbor_encode(&v);
  let err =
    validate_cbor_from_slice(cddl, &bytes, None).expect_err("must fail on wrong-size signature");
  let msg = err.to_string();
  assert!(
    msg.contains("/0/1"),
    "expected deep path /0/1 in error, got:\n{}",
    msg
  );
  assert!(
    msg.contains(".size 64"),
    "expected size constraint mention, got:\n{}",
    msg
  );
}

// Regression: integer literals up to u64::MAX must parse and validate
// correctly on every target (the AST stores them as u64/i128, not usize/isize).
#[test]
fn validate_u64_max_literal_in_range() {
  let cddl_input = r#"
    start = 1 .. 18446744073709551615
  "#;

  // Lower bound.
  let v = Value::Integer(1.into());
  validate_cbor_from_slice(cddl_input, &cbor_encode(&v), None).unwrap();

  // Upper bound = u64::MAX.
  let v = Value::Integer(u64::MAX.into());
  validate_cbor_from_slice(cddl_input, &cbor_encode(&v), None).unwrap();

  // Below the range.
  let v = Value::Integer(0.into());
  assert!(validate_cbor_from_slice(cddl_input, &cbor_encode(&v), None).is_err());

  // Same range via a typename — checks resolve_bound_to_uint over u64.
  let cddl_named = r#"
    start = 1 .. max_u64
    max_u64 = 18446744073709551615
  "#;
  let v = Value::Integer(u64::MAX.into());
  validate_cbor_from_slice(cddl_named, &cbor_encode(&v), None).unwrap();

  // Direct match against the literal.
  let cddl_lit = r#"start = 18446744073709551615"#;
  let v = Value::Integer(u64::MAX.into());
  validate_cbor_from_slice(cddl_lit, &cbor_encode(&v), None).unwrap();
  let v = Value::Integer(0.into());
  assert!(validate_cbor_from_slice(cddl_lit, &cbor_encode(&v), None).is_err());
}

// Regression: `bstr .size N` as a member key must filter Map keys correctly,
// and as a top-level type must reject inputs of the wrong type or wrong size.
#[test]
fn validate_bstr_size_in_member_key_and_top_level() {
  // Top-level: bytes of size 32.
  let cddl_input = r#"start = bstr .size 32"#;
  let bytes_32 = Value::Bytes(vec![0xaa; 32]);
  validate_cbor_from_slice(cddl_input, &cbor_encode(&bytes_32), None).unwrap();

  let bytes_31 = Value::Bytes(vec![0xaa; 31]);
  assert!(validate_cbor_from_slice(cddl_input, &cbor_encode(&bytes_31), None).is_err());

  let not_bytes = Value::Text("xx".to_string());
  assert!(validate_cbor_from_slice(cddl_input, &cbor_encode(&not_bytes), None).is_err());

  // Member key: keys must be bstr; values arbitrary.
  let cddl_map = r#"start = {* bstr .size 32 => uint}"#;
  let valid_map = Value::Map(vec![(
    Value::Bytes(vec![0xbb; 32]),
    Value::Integer(7.into()),
  )]);
  validate_cbor_from_slice(cddl_map, &cbor_encode(&valid_map), None).unwrap();

  // Empty map is also valid under `*`.
  let empty_map = Value::Map(vec![]);
  validate_cbor_from_slice(cddl_map, &cbor_encode(&empty_map), None).unwrap();
}

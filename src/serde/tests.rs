use core::{
  fmt::{Debug, Display},
  str::FromStr,
};
use std::{format, io, string::ToString};

use serde_core::{
  Deserialize, Deserializer, Serialize,
  de::{self, DeserializeOwned, IntoDeserializer, Visitor},
};
use serde_json::error::Category;
use serde_test::{Compact, Readable, Token, assert_de_tokens_error};
use typenum::{U8, U10, U18, U60};

use crate::{Decimal, ParseError, u320};

/// The message of every input that is not a string.
const EXPECTED: &str = "expected a decimal value as a string";

type Price = Decimal<U10, U8>;
type Amount = Decimal<U10, U8, u64>;
type TokenAmount = Decimal<U60, U18, u320>;

/// Asserts that `T` rejects every input but a string with exactly [`EXPECTED`], in a readable and a
/// compact configuration, in JSON and through a deserializer of a plain number, and that a string
/// that does not parse reports its parse error.
fn assert_rejects_all_but_strings<T>()
where
  T: DeserializeOwned + Debug,
{
  for tokens in [
    &[Token::Bool(true)][..],
    &[Token::I8(-7)],
    &[Token::I64(-7_351_946)],
    &[Token::U8(7)],
    &[Token::U64(7_351_946)],
    &[Token::F32(16.7)],
    &[Token::F64(16.7)],
    &[Token::Char('7')],
    &[Token::Bytes(b"7351946")],
    &[Token::BorrowedBytes(b"7351946")],
    &[Token::ByteBuf(b"7351946")],
    &[Token::None],
    &[Token::Some, Token::Str("1.5")],
    &[Token::Unit],
    &[Token::UnitStruct { name: "Amount" }],
    &[Token::NewtypeStruct { name: "Amount" }, Token::Str("1.5")],
    &[Token::Seq { len: Some(1) }],
    &[Token::Map { len: Some(0) }],
  ] {
    assert_de_tokens_error::<Readable<T>>(tokens, EXPECTED);
    assert_de_tokens_error::<Compact<T>>(tokens, EXPECTED);
  }
  for json in [
    "123.45",
    "5",
    "-7351946",
    "18446744073709551616",
    "1e30",
    "true",
    "[1]",
    "{\"v\":1}",
    "null",
  ] {
    let error = serde_json::from_str::<T>(json).unwrap_err();
    assert_eq!(error.to_string(), EXPECTED, "{json}");
    assert_eq!(error.classify(), Category::Data, "{json}");
  }
  let number: de::value::U64Deserializer<de::value::Error> = 7_351_946_u64.into_deserializer();
  assert_eq!(T::deserialize(number).unwrap_err().to_string(), EXPECTED);
  // A string that does not parse reports why, without the input.
  assert_de_tokens_error::<T>(&[Token::Str("7351946x")], "invalid decimal syntax");
}

#[test]
fn every_input_but_a_string_gets_one_fixed_message() {
  assert_rejects_all_but_strings::<Price>();
  assert_rejects_all_but_strings::<Amount>();
  assert_rejects_all_but_strings::<TokenAmount>();
}

/// A reader that yields its bytes and then fails.
struct FailingReader(&'static [u8]);

impl io::Read for FailingReader {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    if self.0.is_empty() {
      return Err(io::Error::new(io::ErrorKind::ConnectionReset, "dropped"));
    }
    let len = buf.len().min(self.0.len());
    let (head, rest) = self.0.split_at(len);
    buf[..len].copy_from_slice(head);
    self.0 = rest;
    Ok(len)
  }
}

/// A struct with a decimal field, whose derived implementation reports missing and unknown fields.
#[derive(Debug, serde_full::Deserialize)]
#[serde(crate = "serde_full", deny_unknown_fields)]
struct Holder<T> {
  amount: T,
  note: Option<u8>,
}

/// Asserts that a format error that names no input passes through with its category: postcard's at
/// a truncated or corrupt string, serde_json's at the end of the input and at a failing reader, and
/// a missing or unknown field of a derived struct; and that a number in a field gets [`EXPECTED`]
/// with the position that serde_json adds. `text` parses as a `T` whose text has at least two
/// bytes.
fn assert_format_errors_pass_through<T>(text: &str)
where
  T: DeserializeOwned + Serialize + FromStr<Err = ParseError> + Display + Debug,
{
  let value: T = text.parse().unwrap();
  let bytes = postcard::to_stdvec(&value).unwrap();
  assert_eq!(
    postcard::from_bytes::<T>(&bytes[..bytes.len() - 1]).unwrap_err(),
    postcard::Error::DeserializeUnexpectedEnd
  );
  let mut corrupt = bytes;
  corrupt[2] = 0xff;
  assert_eq!(
    postcard::from_bytes::<T>(&corrupt).unwrap_err(),
    postcard::Error::DeserializeBadUtf8
  );

  let error = serde_json::from_str::<T>(&format!("\"{text}")).unwrap_err();
  assert_eq!(error.classify(), Category::Eof);
  let error = serde_json::from_reader::<_, T>(FailingReader(b"\"0.0")).unwrap_err();
  assert_eq!(error.classify(), Category::Io);
  assert_eq!(error.io_error_kind(), Some(io::ErrorKind::ConnectionReset));

  let holder: Holder<T> =
    serde_json::from_str(&format!(r#"{{"amount":"{text}","note":null}}"#)).unwrap();
  assert_eq!(holder.amount.to_string(), value.to_string());
  assert_eq!(holder.note, None);
  let error = serde_json::from_str::<Holder<T>>(r#"{"note":1}"#).unwrap_err();
  assert!(
    error.to_string().starts_with("missing field `amount`"),
    "{error}"
  );
  let error = serde_json::from_str::<Holder<T>>(r#"{"amount":"1","extra":1}"#).unwrap_err();
  assert!(
    error.to_string().starts_with("unknown field `extra`"),
    "{error}"
  );
  let error = serde_json::from_str::<Holder<T>>(r#"{"amount":"#).unwrap_err();
  assert_eq!(error.classify(), Category::Eof);
  let error = serde_json::from_str::<Holder<T>>(r#"{"amount":7351946}"#).unwrap_err();
  let message = error.to_string();
  assert!(message.starts_with(EXPECTED), "{message}");
  assert!(message.contains(" at line 1 column "), "{message}");
  assert!(!message.contains("7351946"), "{message}");
}

#[test]
fn a_format_error_that_names_no_input_passes_through_with_its_category() {
  assert_format_errors_pass_through::<Price>("-64250.5");
  assert_format_errors_pass_through::<Amount>("0.00100000");
  assert_format_errors_pass_through::<TokenAmount>("1.000000000000000001");
}

/// A deserializer of one text field in a format that infers the type of the field from its text,
/// as csv's does: a number goes to the visitor as a number unless a string is asked for.
struct InferringField<'de>(&'de str);

impl<'de> Deserializer<'de> for InferringField<'de> {
  type Error = de::value::Error;

  fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
    if let Ok(number) = self.0.parse::<u64>() {
      visitor.visit_u64(number)
    } else if let Ok(number) = self.0.parse::<i64>() {
      visitor.visit_i64(number)
    } else if let Ok(number) = self.0.parse::<f64>() {
      visitor.visit_f64(number)
    } else {
      visitor.visit_borrowed_str(self.0)
    }
  }

  fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
    visitor.visit_borrowed_str(self.0)
  }

  serde_core::forward_to_deserialize_any! {
    bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char string bytes byte_buf option unit
    unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any
  }
}

#[test]
fn a_value_reads_back_through_a_format_that_infers_numbers() {
  for text in ["96.37575", "-64250.5", "5838", "-1", "0"] {
    let price = Price::deserialize(InferringField(text)).unwrap();
    assert_eq!(price.to_string(), text);
  }
}

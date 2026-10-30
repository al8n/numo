use super::{ParseError, ParseErrorKind};

/// Where a valid finite input keeps the digits of its coefficient.
pub(super) struct Digits {
  /// Whether the input starts with `-`.
  pub(super) negative: bool,
  /// Index of the first nonzero digit, or `end` if the value is zero.
  pub(super) start: usize,
  /// One past the index of the last nonzero digit. Bytes between `start` and `end` that are not
  /// digits are `_` or `.`.
  pub(super) end: usize,
  /// The coefficient is the integer formed by the digits between `start` and `end`, times
  /// `10^scale`.
  pub(super) scale: u32,
  /// The scale of the text: the number of fraction digits written, not counting `_`, minus the
  /// exponent, and at least 0; `None` if it is above 255.
  pub(super) text_scale: Option<u8>,
}

/// Validates `input` for a decimal with `integral` integral and `fraction` fraction digits, and
/// negative values only if `signed`, and locates the digits of its coefficient.
///
/// The grammar is the one documented on `parse`. Errors are reported in priority order: `Empty`;
/// `InvalidSyntax` if the whole input does not match the grammar; `NotFinite` for an infinity or a
/// NaN; `Negative` if the input starts with `-` and `signed` is `false`, whatever the magnitude;
/// `OutOfRange` if, after the exponent moves the point, the integral part has more than `integral`
/// digits after its leading zeros; `Inexact` if a nonzero digit then falls beyond the first
/// `fraction` fraction digits. Runs in linear time and constant space.
pub(super) const fn scan(
  input: &[u8],
  integral: u32,
  fraction: u32,
  signed: bool,
) -> Result<Digits, ParseError> {
  let len = input.len();
  if len == 0 {
    return Err(ParseError::new(ParseErrorKind::Empty));
  }

  let negative = input[0] == b'-';
  let mut pos = if negative || input[0] == b'+' { 1 } else { 0 };

  // A finite number starts with a digit or a point; anything else must be an infinity or a NaN.
  if pos < len && !input[pos].is_ascii_digit() && input[pos] != b'.' {
    let kind = if is_nonfinite(input, pos) {
      ParseErrorKind::NotFinite
    } else {
      ParseErrorKind::InvalidSyntax
    };
    return Err(ParseError::new(kind));
  }

  // The mantissa. Digits are counted without the `_` and `.` among them; the significant digits
  // run from the first to the last nonzero digit.
  let mut digits: usize = 0;
  let mut integral_digits = 0;
  let mut seen_point = false;
  // Whether a digit has started the current run, integral or fraction; only then may `_` follow.
  let mut in_run = false;
  let mut leading_zeros = 0;
  let mut significant_end = 0;
  let mut start = 0;
  let mut end = 0;
  while pos < len {
    let byte = input[pos];
    if byte.is_ascii_digit() {
      if byte != b'0' {
        if end == 0 {
          leading_zeros = digits;
          start = pos;
        }
        significant_end = digits + 1;
        end = pos + 1;
      }
      digits += 1;
      in_run = true;
    } else if byte == b'.' && !seen_point {
      integral_digits = digits;
      seen_point = true;
      in_run = false;
    } else if byte != b'_' || !in_run {
      break;
    }
    pos += 1;
  }
  if digits == 0 {
    return Err(ParseError::new(ParseErrorKind::InvalidSyntax));
  }
  if !seen_point {
    integral_digits = digits;
  }

  // The exponent saturates at `u64::MAX`. Inputs have at most `isize::MAX` bytes, so an exponent
  // of that magnitude moves the point more than 154 places past every digit, to either side, and
  // decides range and exactness as any larger exponent would.
  let mut exponent: u64 = 0;
  let mut exponent_negative = false;
  if pos < len && (input[pos] == b'e' || input[pos] == b'E') {
    pos += 1;
    if pos < len && (input[pos] == b'+' || input[pos] == b'-') {
      exponent_negative = input[pos] == b'-';
      pos += 1;
    }
    let mut exponent_digits = false;
    while pos < len {
      let byte = input[pos];
      if byte.is_ascii_digit() {
        exponent = exponent
          .saturating_mul(10)
          .saturating_add((byte - b'0') as u64);
        exponent_digits = true;
      } else if byte != b'_' {
        break;
      }
      pos += 1;
    }
    if !exponent_digits {
      return Err(ParseError::new(ParseErrorKind::InvalidSyntax));
    }
  }
  if pos < len {
    return Err(ParseError::new(ParseErrorKind::InvalidSyntax));
  }

  // The input is a finite number. An unsigned target holds no negative value, not even zero, so
  // the sign is decided before the value.
  if negative && !signed {
    return Err(ParseError::new(ParseErrorKind::Negative));
  }

  // The digit counts are at most `isize::MAX` and the exponent's magnitude at most `u64::MAX`, so
  // `i128` holds every intermediate result below.
  let exponent = if exponent_negative {
    -(exponent as i128)
  } else {
    exponent as i128
  };
  // A saturated exponent gives a scale above 255 or below 0 as the exact one would.
  let text_scale = match (digits - integral_digits) as i128 - exponent {
    scale if scale <= 0 => Some(0),
    scale if scale <= u8::MAX as i128 => Some(scale as u8),
    _ => None,
  };

  if end == 0 {
    // Zero, whatever the sign and the exponent.
    return Ok(Digits {
      negative,
      start,
      end,
      scale: 0,
      text_scale,
    });
  }

  // After the exponent moves it, the point follows the first `point` digits of the mantissa, or
  // precedes them by `-point` zeros. The integral part then has `point - leading_zeros` digits
  // after its leading zeros, and the last nonzero digit is fraction digit
  // `significant_end - point`.
  let point = exponent + integral_digits as i128;
  if point - leading_zeros as i128 > integral as i128 {
    return Err(ParseError::new(ParseErrorKind::OutOfRange));
  }
  let scale = fraction as i128 + point - significant_end as i128;
  if scale < 0 {
    return Err(ParseError::new(ParseErrorKind::Inexact));
  }
  // There is at least one significant digit, so `scale < integral + fraction`.
  Ok(Digits {
    negative,
    start,
    end,
    scale: scale as u32,
    text_scale,
  })
}

/// Returns `true` if `input[start..]` is `inf`, `infinity`, or `nan` or `snan` followed by any
/// number of digits, ignoring ASCII case. Requires `start <= input.len()`.
const fn is_nonfinite(input: &[u8], start: usize) -> bool {
  let len = input.len();
  if starts_with_ignore_case(input, start, b"inf") {
    let rest = len - start;
    return rest == 3 || (rest == 8 && starts_with_ignore_case(input, start, b"infinity"));
  }
  let mut pos = start;
  if pos < len && input[pos].eq_ignore_ascii_case(&b's') {
    pos += 1;
  }
  if !starts_with_ignore_case(input, pos, b"nan") {
    return false;
  }
  pos += 3;
  while pos < len {
    if !input[pos].is_ascii_digit() {
      return false;
    }
    pos += 1;
  }
  true
}

/// Returns `true` if `input[start..]` starts with `word`, ignoring ASCII case. Requires
/// `start <= input.len()`.
const fn starts_with_ignore_case(input: &[u8], start: usize, word: &[u8]) -> bool {
  if input.len() - start < word.len() {
    return false;
  }
  let mut i = 0;
  while i < word.len() {
    if !input[start + i].eq_ignore_ascii_case(&word[i]) {
      return false;
    }
    i += 1;
  }
  true
}

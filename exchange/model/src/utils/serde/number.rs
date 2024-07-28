use serde::de::Error;
use serde::{Deserializer, Serializer};

fn format_to_hex2(buffer: &mut [u8], mut num: i64) -> &str {
    let mut i = buffer.len() - 1;
    let sign = num < 0;
    if sign {
        num = -num;
    }
    const LOOKUP: &[u8] = b"0123456789ABCDEF";
    while num > 0 {
        buffer[i] = LOOKUP[(num & 0xF) as usize];
        num >>= 4;
        i -= 1;
    }
    if i == buffer.len() - 1 {
        buffer[i] = b'0';
        i -= 1;
    }
    if (buffer.len() - i - 1) % 2 != 0 {
        buffer[i] = b'0';
        i -= 1;
    }
    if sign {
        buffer[i] = b'-';
        i -= 1;
    }
    // safety: the buffer is filled with valid utf8
    unsafe { std::str::from_utf8_unchecked(&buffer[i + 1..]) }
}

macro_rules! impl_hex2 {
    ($m: ident, $t: ty, $neg: expr) => {
        pub mod $m {
            use super::*;
            use crate::utils::serde::CowStrVisitor;

            /// Serialize a i64 to a hex string: AB, 1F, -12, etc.
            /// always even number of characters
            pub fn serialize<S>(num: &$t, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let num = *num;
                let mut buffer = [0u8; 16];
                let str = format_to_hex2(&mut buffer, num as i64);
                serializer.serialize_str(str)
            }

            pub fn deserialize<'de, D>(deserializer: D) -> Result<$t, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s = deserializer.deserialize_str(CowStrVisitor)?;
                if s.is_empty() {
                    return Err(Error::invalid_length(0, &"a non-empty string"));
                }
                #[allow(unused_comparisons)]
                if $neg < 0 && s.chars().next().unwrap() == '-' {
                    // safety: the first character is '-'
                    let num = <$t>::from_str_radix(&s[1..], 16).map_err(Error::custom)?;
                    Ok(num * $neg)
                } else {
                    let num = <$t>::from_str_radix(&s, 16).map_err(Error::custom)?;
                    Ok(num)
                }
            }
        }
    };
}
impl_hex2!(hex2_i64, i64, -1);
impl_hex2!(hex2_i32, i32, -1);
impl_hex2!(hex2_i16, i16, -1);
impl_hex2!(hex2_i8, i8, -1);
impl_hex2!(hex2_u64, u64, 1);
impl_hex2!(hex2_u32, u32, 1);
impl_hex2!(hex2_u16, u16, 1);
impl_hex2!(hex2_u8, u8, 1);
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_to_hex2() {
        let mut buffer = [0u8; 16];
        assert_eq!(format_to_hex2(&mut buffer, 0), "00");
        assert_eq!(format_to_hex2(&mut buffer, 1), "01");
        assert_eq!(format_to_hex2(&mut buffer, 15), "0F");
        assert_eq!(format_to_hex2(&mut buffer, 16), "10");
        assert_eq!(format_to_hex2(&mut buffer, 255), "FF");
        assert_eq!(format_to_hex2(&mut buffer, 256), "0100");
        assert_eq!(format_to_hex2(&mut buffer, -1), "-01");
        assert_eq!(format_to_hex2(&mut buffer, -15), "-0F");
        assert_eq!(format_to_hex2(&mut buffer, -16), "-10");
        assert_eq!(format_to_hex2(&mut buffer, -255), "-FF");
        assert_eq!(format_to_hex2(&mut buffer, -256), "-0100");
    }
}

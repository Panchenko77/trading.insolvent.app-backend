use eyre::Context;
use serde::{Deserialize, Serialize};

pub type Value = f64;
pub type Precision = f64;
pub type Decimals = i32;

#[derive(
    Debug, Clone, Copy, PartialOrd, PartialEq, Serialize, Deserialize, parse_display::Display, parse_display::FromStr,
)]
pub enum SizeMode {
    /// Absolute means the number of decimals is fixed. i.e. 1.234's decimals is 3.
    Absolute,
    /// Relative means the number of decimals is relative to the value. i.e. 1.234's decimals is 4.
    Relative,
}

#[derive(
    Debug, Clone, Copy, PartialOrd, PartialEq, Serialize, Deserialize, parse_display::Display, parse_display::FromStr,
)]
pub enum SizeSource {
    Custom,
    Precision,
    // PrecisionStr,
    Decimals,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Size {
    pub precision: Precision,
    pub precision_inverse: Precision,
    pub decimals: Decimals,
    pub mode: SizeMode,
    pub source: SizeSource,
}

impl Size {
    pub const ONE: Self = Self {
        precision: 1.0,
        precision_inverse: 1.0,
        decimals: 0,
        mode: SizeMode::Absolute,
        source: SizeSource::Custom,
    };
    pub const PRECISE: Self = Self {
        precision: 0.00000001,
        precision_inverse: 100000000.0,
        decimals: 8,
        mode: SizeMode::Absolute,
        source: SizeSource::Custom,
    };
    pub fn from_precision(precision: Precision) -> Self {
        let decimals = convert_precision_to_decimals(precision);

        Self {
            precision,
            precision_inverse: 1.0 / precision,
            decimals,
            mode: SizeMode::Absolute,
            source: SizeSource::Precision,
        }
    }
    pub fn from_precision_str(precision: &str) -> eyre::Result<Self> {
        let decimals = convert_precision_str_to_decimals(precision);
        let precision = precision
            .parse()
            .with_context(|| format!("invalid precision: {}", precision))?;
        Ok(Self {
            precision,
            precision_inverse: 1.0 / precision,
            decimals,
            mode: SizeMode::Absolute,
            // source: ScaleSource::PrecisionStr,
            source: SizeSource::Precision,
        })
    }
    pub fn from_decimals(decimals: Decimals) -> Self {
        let precision = convert_decimals_to_precision(decimals);
        let mode = SizeMode::Absolute;
        Self {
            precision,
            precision_inverse: 1.0 / precision,
            decimals,
            mode,
            source: SizeSource::Decimals,
        }
    }
    pub fn inverse(&self) -> Self {
        Self {
            precision: self.precision_inverse,
            precision_inverse: self.precision,
            decimals: -self.decimals,
            mode: self.mode,
            source: self.source,
        }
    }
    pub fn with_mode(mut self, mode: SizeMode) -> Self {
        self.mode = mode;
        self
    }
    pub fn precision_by(&self, value: Value) -> Precision {
        match self.mode {
            SizeMode::Absolute => self.precision,
            SizeMode::Relative => convert_relative_decimals_to_precision(self.decimals, value),
        }
    }
    pub fn _round_to_decimals(&self, value: Value) -> f64 {
        round_to_decimals_by_mode(value, self.decimals, self.mode)
    }
    pub fn round_to_decimals(&self, value: Value) -> f64 {
        debug_assert_eq!(self.source, SizeSource::Decimals);
        self._round_to_decimals(value)
    }
    pub fn _round_to_precision(&self, value: Value) -> f64 {
        round_to_precision(value, self.precision)
    }
    pub fn round_to_precision(&self, value: Value) -> f64 {
        debug_assert_eq!(self.source, SizeSource::Precision);
        self._round_to_precision(value)
    }
    pub fn round(&self, value: Value) -> f64 {
        match self.source {
            SizeSource::Custom => panic!("Can't use round directly when SizeSource::Custom "),
            SizeSource::Precision => self._round_to_precision(value),
            SizeSource::Decimals => self._round_to_decimals(value),
        }
    }
    pub fn _format_with_decimals_absolute(&self, value: Value) -> String {
        format_quantity_with_decimals(value, self.decimals)
    }
    pub fn format_with_decimals_absolute(&self, value: Value) -> String {
        debug_assert_eq!(self.source, SizeSource::Decimals);
        self._format_with_decimals_absolute(value)
    }
    pub fn _format_with_significant_digits(&self, value: Value) -> String {
        format_quantity_with_significant_digits(value, self.decimals)
    }
    pub fn format_with_significant_digits(&self, value: Value) -> String {
        debug_assert_eq!(self.source, SizeSource::Decimals);
        debug_assert_eq!(self.mode, SizeMode::Relative);
        self._format_with_significant_digits(value)
    }
    pub fn _format_with_decimals(&self, value: Value) -> String {
        match self.mode {
            SizeMode::Absolute => self._format_with_decimals_absolute(value),
            SizeMode::Relative => self._format_with_significant_digits(value),
        }
    }
    pub fn format_with_decimals(&self, value: Value) -> String {
        debug_assert_eq!(self.source, SizeSource::Decimals);
        self._format_with_decimals(value)
    }

    pub fn _format_with_precision(&self, value: Value) -> String {
        format_quantity_with_precision(value, self.precision)
    }
    pub fn format_with_precision(&self, value: Value) -> String {
        debug_assert_eq!(self.source, SizeSource::Precision);
        debug_assert_eq!(self.mode, SizeMode::Absolute);
        self._format_with_precision(value)
    }
    pub fn format(&self, value: Value) -> String {
        match self.source {
            SizeSource::Custom => panic!("Can't use format directly when ScaleSource::Custom "),
            SizeSource::Precision => self._format_with_precision(value),
            SizeSource::Decimals => self._format_with_decimals(value),
        }
    }
    pub fn multiple_of(&self, value: Value) -> Value {
        value * self.precision_inverse
    }
    pub fn multiply(&self, value: Value) -> Value {
        value * self.precision
    }
}

pub fn extract_biggest_digits(value: Value) -> Decimals {
    1 + value.log10().floor() as Decimals
}

pub fn convert_precision_to_decimals(precision: Precision) -> Decimals {
    -precision.log10().floor() as Decimals
}

pub fn convert_precision_str_to_decimals(precision: &str) -> Decimals {
    if precision == "1" {
        return 0;
    }
    if precision == "1.0" {
        return 0;
    }
    if precision.starts_with("0.") {
        // forms like 0.0001
        precision.len() as i32 - 2
    } else {
        // forms like 10.0
        -(precision.len() as i32 - 3)
    }
}

pub fn convert_decimals_to_precision(decimals: Decimals) -> Value {
    10.0f64.powi(-decimals)
}

/// e.g. price = 1.2345, price_decimals = 5, tick_size = 0.0001
/// e.g. price = 123.45, price_decimals = 5, tick_size = 0.01
pub fn convert_relative_decimals_to_precision(decimals: Decimals, value: Value) -> f64 {
    let digits = extract_biggest_digits(value);
    let precision = convert_decimals_to_precision(decimals - digits);
    precision
}

pub fn format_quantity_with_decimals(value: Value, decimals: Decimals) -> String {
    if decimals >= 0 {
        format!("{:.*}", decimals as usize, value)
    } else {
        let precision = convert_decimals_to_precision(decimals);
        let value = (value / precision).round() * precision;
        format!("{:.0}", value)
    }
}

pub fn format_quantity_with_precision(value: Value, precision: Precision) -> String {
    let decimals = convert_precision_to_decimals(precision);
    let quantity = (value / precision).round() * precision;
    format_quantity_with_decimals(quantity, decimals)
}

// TODO: this is probably very slow, can we optimize it with a lookup table?
pub fn format_quantity_with_significant_digits(num: f64, digits: i32) -> String {
    let a = num.abs();
    let digits = if a > 1. {
        let n = (1. + a.log10().floor()) as i32;
        if n <= digits {
            digits - n
        } else {
            0
        }
    } else if a > 0. {
        let n = -(1. + a.log10().floor()) as i32;
        digits + n
    } else {
        0
    } as usize;

    format!("{:.*}", digits, num)
}

pub fn extract_decimals_from_string(input: &str) -> i32 {
    if let Some(decimal_point_index) = input.find('.') {
        // Check if there are characters after the decimal point
        if decimal_point_index < input.len() - 1 {
            // Count the number of digits after the decimal point
            let decimal_places_count = input[decimal_point_index + 1..]
                .chars()
                .take_while(|c| c.is_digit(10))
                .count() as i32;

            decimal_places_count
        } else {
            // No characters after the decimal point
            0
        }
    } else {
        // No decimal point found
        0
    }
}

pub fn round_to_decimals(value: Value, decimals: Decimals) -> Value {
    let precision = 10.0f64.powi(decimals);
    (value * precision).round() / precision
}

pub fn round_to_decimals_relative(value: Value, decimals: Decimals) -> Value {
    let digits = extract_biggest_digits(value);
    let precision = convert_decimals_to_precision(decimals - digits);
    (value / precision).round() * precision
}

pub fn round_to_decimals_by_mode(value: Value, decimals: Decimals, mode: SizeMode) -> f64 {
    match mode {
        SizeMode::Absolute => round_to_decimals(value, decimals),
        SizeMode::Relative => round_to_decimals_relative(value, decimals),
    }
}

pub fn round_to_precision(value: Value, precision: Precision) -> f64 {
    let decimals = convert_precision_to_decimals(precision);
    round_to_decimals(value, decimals)
}

pub fn round_by_mode(value: Value, decimals: Decimals, precision: Precision, mode: SizeMode) -> f64 {
    match mode {
        SizeMode::Absolute => round_to_precision(value, precision),
        SizeMode::Relative => round_to_decimals_relative(value, decimals),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_eq::assert_float_eq;

    const ABS: f64 = 1e-8;

    #[test]
    fn test_convert_precision_to_decimals() {
        assert_eq!(convert_precision_to_decimals(10.0), -1);
        assert_eq!(convert_precision_to_decimals(1.0), 0);
        assert_eq!(convert_precision_to_decimals(0.0001), 4);
        assert_eq!(convert_precision_to_decimals(0.00001), 5);
    }

    #[test]
    fn test_convert_precision_str_to_decimals() {
        assert_eq!(convert_precision_str_to_decimals("10.0"), -1);
        assert_eq!(convert_precision_str_to_decimals("1.0"), 0);
        assert_eq!(convert_precision_str_to_decimals("0.0001"), 4);
        assert_eq!(convert_precision_str_to_decimals("0.00001"), 5);
    }

    #[test]
    fn test_convert_decimals_to_precision() {
        assert_float_eq!(convert_decimals_to_precision(0), 1.0, abs <= ABS);
        assert_float_eq!(convert_decimals_to_precision(1), 0.1, abs <= ABS);
        assert_float_eq!(convert_decimals_to_precision(4), 0.0001, abs <= ABS);
        assert_float_eq!(convert_decimals_to_precision(5), 0.00001, abs <= ABS);
    }

    #[test]
    fn test_format_quantity_with_decimals() {
        assert_eq!(format_quantity_with_decimals(1.0, 0), "1");
        assert_eq!(format_quantity_with_decimals(1.0, 1), "1.0");
        assert_eq!(format_quantity_with_decimals(10.0, 0), "10");
        assert_eq!(format_quantity_with_decimals(10.0, -1), "10");
    }

    #[test]
    fn test_format_quantity_with_significant_digits() {
        assert_eq!(format_quantity_with_significant_digits(0.000456, 2), "0.00046");
        assert_eq!(format_quantity_with_significant_digits(0.043256, 3), "0.0433");
        assert_eq!(format_quantity_with_significant_digits(0.01, 2), "0.010");
        assert_eq!(format_quantity_with_significant_digits(10., 3), "10.0");
        assert_eq!(format_quantity_with_significant_digits(456.789, 4), "456.8");
    }

    #[test]
    fn test_extract_decimal_places_with_decimal() {
        assert_eq!(extract_decimals_from_string("123.456"), 3);
        assert_eq!(extract_decimals_from_string("0.001"), 3);
        assert_eq!(extract_decimals_from_string("987654.321"), 3);
        assert_eq!(extract_decimals_from_string("3.14"), 2);
    }

    #[test]
    fn test_extract_decimal_places_without_decimal() {
        assert_eq!(extract_decimals_from_string("123"), 0);
        assert_eq!(extract_decimals_from_string("0"), 0);
        assert_eq!(extract_decimals_from_string("987654"), 0);
    }

    #[test]
    fn test_extract_decimal_places_empty_input() {
        assert_eq!(extract_decimals_from_string(""), 0);
    }

    #[test]
    fn test_extract_decimal_places_decimal_at_end() {
        assert_eq!(extract_decimals_from_string("42."), 0);
        assert_eq!(extract_decimals_from_string("0."), 0);
        assert_eq!(extract_decimals_from_string("123456."), 0);
    }

    #[test]
    fn test_round_to_decimals() {
        assert_float_eq!(round_to_decimals(1.23456789, 0), 1.0, abs <= ABS);
        assert_float_eq!(round_to_decimals(1.23456789, 1), 1.2, abs <= ABS);
        assert_float_eq!(round_to_decimals(1.23456789, 2), 1.23, abs <= ABS);
        assert_float_eq!(round_to_decimals(1.23456789, 3), 1.235, abs <= ABS);
    }

    #[test]
    fn test_round_to_decimals_relative() {
        assert_float_eq!(round_to_decimals_relative(1.23456789, 1), 1.0, abs <= ABS);
        assert_float_eq!(round_to_decimals_relative(1.23456789, 2), 1.2, abs <= ABS);
        assert_float_eq!(round_to_decimals_relative(1.23456789, 3), 1.23, abs <= ABS);
        assert_float_eq!(round_to_decimals_relative(1.23456789, 4), 1.235, abs <= ABS);

        assert_float_eq!(round_to_decimals_relative(12.3456789, 1), 10.0, abs <= ABS);
        assert_float_eq!(round_to_decimals_relative(12.3456789, 2), 12.0, abs <= ABS);
        assert_float_eq!(round_to_decimals_relative(12.3456789, 3), 12.3, abs <= ABS);
        assert_float_eq!(round_to_decimals_relative(12.3456789, 4), 12.35, abs <= ABS);
    }

    #[test]
    fn test_round_to_precision() {
        assert_float_eq!(round_to_precision(1.23456789, 0.1), 1.2, abs <= ABS);
        assert_float_eq!(round_to_precision(1.23456789, 0.01), 1.23, abs <= ABS);
        assert_float_eq!(round_to_precision(1.23456789, 0.001), 1.235, abs <= ABS);
    }
}

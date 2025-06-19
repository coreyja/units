use std::str::FromStr;
use uom::si::f64::*;
use uom::si::{length, mass, thermodynamic_temperature as temperature, volume};

#[derive(Debug, PartialEq)]
pub enum ConversionError {
    InvalidInputFormat,
    UnknownUnit(String),
    IncompatibleUnits { from: String, to: String },
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::InvalidInputFormat => write!(f, "Error: Invalid input format"),
            ConversionError::UnknownUnit(unit) => write!(f, "Error: Unknown unit '{}'", unit),
            ConversionError::IncompatibleUnits { from, to } => {
                write!(f, "Error: Cannot convert from {} to {}", from, to)
            }
        }
    }
}

impl std::error::Error for ConversionError {}

#[derive(Debug, PartialEq)]
enum UnitType {
    Length,
    Mass,
    Temperature,
    Volume,
}

#[derive(Debug)]
struct ParsedInput {
    value: f64,
    unit: String,
}

fn parse_input(input: &str) -> Result<ParsedInput, ConversionError> {
    let trimmed = input.trim();
    
    // Split by whitespace
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    
    if parts.len() != 2 {
        return Err(ConversionError::InvalidInputFormat);
    }
    
    // Parse the numeric value
    let value = f64::from_str(parts[0])
        .map_err(|_| ConversionError::InvalidInputFormat)?;
    
    Ok(ParsedInput {
        value,
        unit: parts[1].to_lowercase(),
    })
}

fn get_unit_type(unit: &str) -> Option<UnitType> {
    match unit {
        "meter" | "meters" | "foot" | "feet" | "kilometer" | "kilometers" | "mile" | "miles" => Some(UnitType::Length),
        "kilogram" | "kilograms" | "pound" | "pounds" => Some(UnitType::Mass),
        "celsius" | "fahrenheit" => Some(UnitType::Temperature),
        "liter" | "liters" | "gallon" | "gallons" => Some(UnitType::Volume),
        _ => None,
    }
}

fn format_output(value: f64, unit: &str) -> String {
    // Handle zero special case
    if value == 0.0 {
        return format!("0 {}", get_plural_unit(unit, true));
    }
    
    // Check if value is very close to 1 (within floating point precision)
    if (value - 1.0).abs() < 5e-6 {
        return format!("1 {}", get_plural_unit(unit, false));
    }
    
    // Format with 6 significant figures
    let formatted = if value.abs() >= 1.0 {
        // For values >= 1, calculate decimal places needed for 6 sig figs
        let int_digits = (value.abs().log10().floor() + 1.0) as usize;
        let decimal_places = if int_digits >= 6 { 0 } else { 6 - int_digits };
        format!("{:.prec$}", value, prec = decimal_places)
    } else {
        // For values < 1, format with enough decimals
        let mut s = format!("{:.10}", value);
        // Count significant figures after decimal point
        let mut sig_figs = 0;
        let mut found_nonzero = false;
        let mut _decimal_places = 0;
        
        for (i, c) in s.chars().enumerate() {
            if c == '.' {
                continue;
            }
            if i > s.find('.').unwrap() {
                _decimal_places += 1;
                if c != '0' || found_nonzero {
                    found_nonzero = true;
                    sig_figs += 1;
                    if sig_figs >= 6 {
                        s.truncate(i + 1);
                        break;
                    }
                }
            }
        }
        s
    };
    
    // Remove trailing zeros and decimal point if not needed
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    
    // Parse the value to check if it's exactly 1
    // Check if the value is very close to 1.0 (within floating point precision)
    let is_singular = (value - 1.0).abs() < 1e-10 || trimmed == "1";
    
    format!("{} {}", trimmed, get_plural_unit(unit, !is_singular))
}

fn get_plural_unit(unit: &str, plural: bool) -> &str {
    if plural {
        match unit {
            "meter" => "meters",
            "foot" => "feet",
            "kilometer" => "kilometers",
            "mile" => "miles",
            "kilogram" => "kilograms",
            "pound" => "pounds",
            "liter" => "liters",
            "gallon" => "gallons",
            _ => unit,
        }
    } else {
        match unit {
            "meters" => "meter",
            "feet" => "foot",
            "kilometers" => "kilometer",
            "miles" => "mile",
            "kilograms" => "kilogram",
            "pounds" => "pound",
            "liters" => "liter",
            "gallons" => "gallon",
            _ => unit,
        }
    }
}

pub fn convert_units(input: &str, output_unit: &str) -> Result<String, ConversionError> {
    // Parse input
    let parsed = parse_input(input)?;
    
    let output_unit_lower = output_unit.trim().to_lowercase();
    
    // Check if units exist
    let input_type = match get_unit_type(&parsed.unit) {
        Some(t) => t,
        None => return Err(ConversionError::UnknownUnit(parsed.unit)),
    };
    
    let output_type = match get_unit_type(&output_unit_lower) {
        Some(t) => t,
        None => return Err(ConversionError::UnknownUnit(output_unit_lower)),
    };
    
    // Check if units are compatible
    if input_type != output_type {
        let type_name = |t: &UnitType| match t {
            UnitType::Length => "length",
            UnitType::Mass => "mass",
            UnitType::Temperature => "temperature",
            UnitType::Volume => "volume",
        };
        return Err(ConversionError::IncompatibleUnits {
            from: type_name(&input_type).to_string(),
            to: type_name(&output_type).to_string(),
        });
    }
    
    // Perform conversion based on type
    let result = match input_type {
        UnitType::Length => convert_length(parsed.value, &parsed.unit, &output_unit_lower),
        UnitType::Mass => convert_mass(parsed.value, &parsed.unit, &output_unit_lower),
        UnitType::Temperature => convert_temperature(parsed.value, &parsed.unit, &output_unit_lower),
        UnitType::Volume => convert_volume(parsed.value, &parsed.unit, &output_unit_lower),
    };
    
    Ok(format_output(result, &output_unit_lower))
}

fn convert_length(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    let length = match from_unit {
        "meter" | "meters" => Length::new::<length::meter>(value),
        "foot" | "feet" => Length::new::<length::foot>(value),
        "kilometer" | "kilometers" => Length::new::<length::kilometer>(value),
        "mile" | "miles" => Length::new::<length::mile>(value),
        _ => unreachable!(),
    };
    
    match to_unit {
        "meter" | "meters" => length.get::<length::meter>(),
        "foot" | "feet" => length.get::<length::foot>(),
        "kilometer" | "kilometers" => length.get::<length::kilometer>(),
        "mile" | "miles" => length.get::<length::mile>(),
        _ => unreachable!(),
    }
}

fn convert_mass(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    let mass = match from_unit {
        "kilogram" | "kilograms" => Mass::new::<mass::kilogram>(value),
        "pound" | "pounds" => Mass::new::<mass::pound>(value),
        _ => unreachable!(),
    };
    
    match to_unit {
        "kilogram" | "kilograms" => mass.get::<mass::kilogram>(),
        "pound" | "pounds" => mass.get::<mass::pound>(),
        _ => unreachable!(),
    }
}

fn convert_temperature(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    let temp = match from_unit {
        "celsius" => ThermodynamicTemperature::new::<temperature::degree_celsius>(value),
        "fahrenheit" => ThermodynamicTemperature::new::<temperature::degree_fahrenheit>(value),
        _ => unreachable!(),
    };
    
    match to_unit {
        "celsius" => temp.get::<temperature::degree_celsius>(),
        "fahrenheit" => temp.get::<temperature::degree_fahrenheit>(),
        _ => unreachable!(),
    }
}

fn convert_volume(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    let volume = match from_unit {
        "liter" | "liters" => Volume::new::<volume::liter>(value),
        "gallon" | "gallons" => Volume::new::<volume::gallon>(value),
        _ => unreachable!(),
    };
    
    match to_unit {
        "liter" | "liters" => volume.get::<volume::liter>(),
        "gallon" | "gallons" => volume.get::<volume::gallon>(),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meters_to_feet() {
        assert_eq!(convert_units("1 meter", "feet").unwrap(), "3.28084 feet");
        assert_eq!(convert_units("10 meters", "feet").unwrap(), "32.8084 feet");
        assert_eq!(convert_units("0.5 meters", "feet").unwrap(), "1.64042 feet");
    }

    #[test]
    fn test_feet_to_meters() {
        assert_eq!(convert_units("1 foot", "meters").unwrap(), "0.3048 meters");
        assert_eq!(convert_units("10 feet", "meters").unwrap(), "3.048 meters");
        assert_eq!(convert_units("3.28084 feet", "meters").unwrap(), "1 meter");
    }

    #[test]
    fn test_kilometers_to_miles() {
        assert_eq!(convert_units("1 kilometer", "miles").unwrap(), "0.621371 miles");
        assert_eq!(convert_units("10 kilometers", "miles").unwrap(), "6.21371 miles");
        assert_eq!(convert_units("1.60934 kilometers", "miles").unwrap(), "1 mile");
    }

    #[test]
    fn test_miles_to_kilometers() {
        assert_eq!(convert_units("1 mile", "kilometers").unwrap(), "1.60934 kilometers");
        assert_eq!(convert_units("10 miles", "kilometers").unwrap(), "16.0934 kilometers");
        assert_eq!(convert_units("0.621371 miles", "kilometers").unwrap(), "1 kilometer");
    }

    #[test]
    fn test_celsius_to_fahrenheit() {
        assert_eq!(convert_units("0 celsius", "fahrenheit").unwrap(), "32 fahrenheit");
        assert_eq!(convert_units("100 celsius", "fahrenheit").unwrap(), "212 fahrenheit");
        assert_eq!(convert_units("-40 celsius", "fahrenheit").unwrap(), "-40 fahrenheit");
        assert_eq!(convert_units("37 celsius", "fahrenheit").unwrap(), "98.6 fahrenheit");
    }

    #[test]
    fn test_fahrenheit_to_celsius() {
        assert_eq!(convert_units("32 fahrenheit", "celsius").unwrap(), "0 celsius");
        assert_eq!(convert_units("212 fahrenheit", "celsius").unwrap(), "100 celsius");
        assert_eq!(convert_units("-40 fahrenheit", "celsius").unwrap(), "-40 celsius");
        assert_eq!(convert_units("98.6 fahrenheit", "celsius").unwrap(), "37 celsius");
    }

    #[test]
    fn test_kilograms_to_pounds() {
        assert_eq!(convert_units("1 kilogram", "pounds").unwrap(), "2.20462 pounds");
        assert_eq!(convert_units("10 kilograms", "pounds").unwrap(), "22.0462 pounds");
        assert_eq!(convert_units("0.453592 kilograms", "pounds").unwrap(), "1 pound");
    }

    #[test]
    fn test_pounds_to_kilograms() {
        assert_eq!(convert_units("1 pound", "kilograms").unwrap(), "0.453592 kilograms");
        assert_eq!(convert_units("10 pounds", "kilograms").unwrap(), "4.53592 kilograms");
        assert_eq!(convert_units("2.20462 pounds", "kilograms").unwrap(), "1 kilogram");
    }

    #[test]
    fn test_liters_to_gallons() {
        assert_eq!(convert_units("1 liter", "gallons").unwrap(), "0.264172 gallons");
        assert_eq!(convert_units("10 liters", "gallons").unwrap(), "2.64172 gallons");
        assert_eq!(convert_units("3.78541 liters", "gallons").unwrap(), "1 gallon");
    }

    #[test]
    fn test_gallons_to_liters() {
        assert_eq!(convert_units("1 gallon", "liters").unwrap(), "3.78541 liters");
        assert_eq!(convert_units("10 gallons", "liters").unwrap(), "37.8541 liters");
        assert_eq!(convert_units("0.264172 gallons", "liters").unwrap(), "1 liter");
    }

    #[test]
    fn test_invalid_unit() {
        assert_eq!(convert_units("1 invalid_unit", "meters").unwrap_err().to_string(), "Error: Unknown unit 'invalid_unit'");
        assert_eq!(convert_units("1 meter", "invalid_unit").unwrap_err().to_string(), "Error: Unknown unit 'invalid_unit'");
    }

    #[test]
    fn test_incompatible_units() {
        assert_eq!(convert_units("1 meter", "kilograms").unwrap_err().to_string(), "Error: Cannot convert from length to mass");
        assert_eq!(convert_units("1 celsius", "meters").unwrap_err().to_string(), "Error: Cannot convert from temperature to length");
        assert_eq!(convert_units("1 liter", "fahrenheit").unwrap_err().to_string(), "Error: Cannot convert from volume to temperature");
    }

    #[test]
    fn test_invalid_input_format() {
        assert_eq!(convert_units("meter", "feet").unwrap_err().to_string(), "Error: Invalid input format");
        assert_eq!(convert_units("1", "meters").unwrap_err().to_string(), "Error: Invalid input format");
        assert_eq!(convert_units("", "meters").unwrap_err().to_string(), "Error: Invalid input format");
        assert_eq!(convert_units("1 2 meters", "feet").unwrap_err().to_string(), "Error: Invalid input format");
    }

    #[test]
    fn test_plural_units() {
        assert_eq!(convert_units("2 meters", "feet").unwrap(), "6.56168 feet");
        assert_eq!(convert_units("5 feet", "meters").unwrap(), "1.524 meters");
        assert_eq!(convert_units("3 kilograms", "pounds").unwrap(), "6.61387 pounds");
    }

    #[test]
    fn test_decimal_values() {
        assert_eq!(convert_units("1.5 meters", "feet").unwrap(), "4.92126 feet");
        assert_eq!(convert_units("2.5 kilograms", "pounds").unwrap(), "5.51156 pounds");
        assert_eq!(convert_units("0.25 gallons", "liters").unwrap(), "0.946353 liters");
    }

    #[test]
    fn test_negative_values() {
        assert_eq!(convert_units("-5 meters", "feet").unwrap(), "-16.4042 feet");
        assert_eq!(convert_units("-10 celsius", "fahrenheit").unwrap(), "14 fahrenheit");
        assert_eq!(convert_units("-2.5 kilograms", "pounds").unwrap(), "-5.51156 pounds");
    }

    #[test]
    fn test_zero_values() {
        assert_eq!(convert_units("0 meters", "feet").unwrap(), "0 feet");
        assert_eq!(convert_units("0 kilograms", "pounds").unwrap(), "0 pounds");
        assert_eq!(convert_units("0 liters", "gallons").unwrap(), "0 gallons");
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(convert_units("1 METER", "FEET").unwrap(), "3.28084 feet");
        assert_eq!(convert_units("1 Meter", "Feet").unwrap(), "3.28084 feet");
        assert_eq!(convert_units("1 mEtEr", "FeEt").unwrap(), "3.28084 feet");
    }

    #[test]
    fn test_whitespace_handling() {
        assert_eq!(convert_units("  1 meter  ", "feet").unwrap(), "3.28084 feet");
        assert_eq!(convert_units("1  meter", "feet").unwrap(), "3.28084 feet");
        assert_eq!(convert_units("1 meter", "  feet  ").unwrap(), "3.28084 feet");
    }
}

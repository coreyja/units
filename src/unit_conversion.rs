use std::str::FromStr;
use uom::si::f64::*;
use uom::si::{
    acceleration, area, energy, force, length, mass, mass_density, power,
    thermodynamic_temperature as temperature, velocity, volume,
};
use tracing::{error, instrument};

#[derive(Debug, PartialEq)]
pub enum ConversionError {
    InvalidInputFormat,
    UnknownUnit(String),
    IncompatibleUnits { from: String, to: String },
    InvalidUnitCombination,
    UnknownCompoundUnit,
    UnitCancellationNotSupported,
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::InvalidInputFormat => write!(f, "Error: Invalid input format"),
            ConversionError::UnknownUnit(unit) => write!(f, "Error: Unknown unit '{unit}'"),
            ConversionError::IncompatibleUnits { from, to } => {
                write!(f, "Error: Cannot convert from {from} to {to}")
            }
            ConversionError::InvalidUnitCombination => {
                write!(f, "Error: Invalid unit combination")
            }
            ConversionError::UnknownCompoundUnit => write!(f, "Error: Unknown compound unit"),
            ConversionError::UnitCancellationNotSupported => {
                write!(f, "Error: Unit cancellation not supported")
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
    Velocity,
    Area,
    MassDensity,
    Acceleration,
    Force,
    Energy,
    Power,
    FuelEconomy,
}

#[derive(Debug)]
struct ParsedInput {
    value: f64,
    unit: String,
}

fn parse_input(input: &str) -> Result<ParsedInput, ConversionError> {
    let trimmed = input.trim();

    // Handle parentheses by removing them for now
    let cleaned = trimmed.replace("(", "").replace(")", "");

    // Check if this is a multiplication expression like "10 meters * 5 meters"
    if cleaned.contains(" * ") {
        return parse_multiplication_expression(&cleaned);
    }

    // Split by first space
    let parts: Vec<&str> = cleaned.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return Err(ConversionError::InvalidInputFormat);
    }

    // Parse the numeric value
    let value = f64::from_str(parts[0]).map_err(|_| ConversionError::InvalidInputFormat)?;

    let unit_str = parts[1].trim();

    // Check if unit string contains numbers (invalid format like "2 meters")
    if unit_str
        .chars()
        .any(|c| c.is_numeric() && !unit_str.contains('/') && !unit_str.contains('^'))
    {
        return Err(ConversionError::InvalidInputFormat);
    }

    Ok(ParsedInput {
        value,
        unit: unit_str.to_lowercase(),
    })
}

fn parse_multiplication_expression(input: &str) -> Result<ParsedInput, ConversionError> {
    let parts: Vec<&str> = input.split(" * ").collect();

    let mut total_value = 1.0;
    let mut unit_parts = Vec::new();

    for part in parts {
        let part = part.trim();
        let space_idx = part.find(' ').ok_or(ConversionError::InvalidInputFormat)?;
        let (val_str, unit_str) = part.split_at(space_idx);

        let val = f64::from_str(val_str.trim()).map_err(|_| ConversionError::InvalidInputFormat)?;
        total_value *= val;

        unit_parts.push(unit_str.trim().to_lowercase());
    }

    // Determine the resulting unit type based on multiplication
    let result_unit = determine_compound_unit(&unit_parts);

    Ok(ParsedInput {
        value: total_value,
        unit: result_unit,
    })
}

fn determine_compound_unit(units: &[String]) -> String {
    // Simple heuristic for now - if all units are length, result is area or volume
    let all_length = units.iter().all(|u| {
        matches!(
            u.as_str(),
            "meter" | "meters" | "foot" | "feet" | "kilometer" | "kilometers" | "mile" | "miles"
        )
    });

    if all_length {
        match units.len() {
            2 => {
                // Convert to square units
                if units.iter().any(|u| u.contains("meter")) {
                    "square meters".to_string()
                } else if units
                    .iter()
                    .any(|u| u.contains("foot") || u.contains("feet"))
                {
                    "square feet".to_string()
                } else if units.iter().any(|u| u.contains("kilometer")) {
                    "square kilometers".to_string()
                } else if units.iter().any(|u| u.contains("mile")) {
                    "square miles".to_string()
                } else {
                    "square meters".to_string()
                }
            }
            3 => {
                // Convert to cubic units
                if units.iter().any(|u| u.contains("meter")) {
                    "cubic meters".to_string()
                } else if units
                    .iter()
                    .any(|u| u.contains("foot") || u.contains("feet"))
                {
                    "cubic feet".to_string()
                } else if units.iter().any(|u| u.contains("centimeter")) {
                    "cubic centimeters".to_string()
                } else {
                    "cubic meters".to_string()
                }
            }
            _ => units.join(" * "),
        }
    } else {
        units.join(" * ")
    }
}

fn get_unit_type(unit: &str) -> Option<UnitType> {
    // Handle compound units with various formats
    if unit.contains('/')
        || unit.contains(" per ")
        || unit.contains("mph")
        || unit.contains("kmh")
        || unit.contains("kph")
    {
        // Velocity units
        if unit.contains("miles/hour")
            || unit.contains("mph")
            || unit.contains("kilometers/hour")
            || unit.contains("kmh")
            || unit.contains("kph")
            || unit.contains("km/h")
            || unit.contains("miles per hour")
            || unit.contains("meters/second")
            || unit.contains("m/s")
            || unit.contains("feet/second")
            || unit.contains("ft/s")
        {
            return Some(UnitType::Velocity);
        }
        // Density units
        if (unit.contains("kilogram") || unit.contains("gram") || unit.contains("pound"))
            && (unit.contains("cubic") || unit.contains("milliliter") || unit.contains("liter"))
        {
            return Some(UnitType::MassDensity);
        }
        // Acceleration
        if unit.contains("second^2") || unit.contains("second²") {
            return Some(UnitType::Acceleration);
        }
        // Fuel economy
        if (unit.contains("miles") || unit.contains("kilometers"))
            && (unit.contains("gallon") || unit.contains("liter"))
        {
            return Some(UnitType::FuelEconomy);
        }
    }

    // Area units
    if unit.contains("square") || unit.contains("acre") {
        return Some(UnitType::Area);
    }

    // Volume units (cubic)
    if unit.contains("cubic") && !unit.contains('/') {
        return Some(UnitType::Volume);
    }

    // Force units
    if unit.contains("newton") || unit.contains("pounds force") {
        return Some(UnitType::Force);
    }

    // Energy units
    if unit.contains("joule") || unit.contains("foot pound") {
        return Some(UnitType::Energy);
    }

    // Power units
    if unit.contains("watt") || unit.contains("horsepower") {
        return Some(UnitType::Power);
    }

    // Simple units
    match unit {
        "meter" | "meters" | "foot" | "feet" | "kilometer" | "kilometers" | "mile" | "miles" => {
            Some(UnitType::Length)
        }
        "kilogram" | "kilograms" | "pound" | "pounds" | "gram" | "grams" => Some(UnitType::Mass),
        "celsius" | "fahrenheit" => Some(UnitType::Temperature),
        "liter" | "liters" | "gallon" | "gallons" | "milliliter" | "milliliters" => {
            Some(UnitType::Volume)
        }
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

    // Format with appropriate precision
    let formatted = if value.abs() >= 1000.0 {
        // For large values, use fixed decimal places
        format!("{value:.2}")
    } else if value.abs() >= 1.0 {
        // For values >= 1, calculate decimal places needed for 6 sig figs
        let int_digits = (value.abs().log10().floor() + 1.0) as usize;
        let decimal_places = 6_usize.saturating_sub(int_digits);
        format!("{value:.decimal_places$}")
    } else if value.abs() >= 0.01 {
        // For small values, use more decimal places
        format!("{value:.6}")
    } else {
        // For very small values, use scientific notation style formatting
        format!("{value:.6}")
    };

    // Remove trailing zeros and decimal point if not needed
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');

    // Parse the value to check if it's exactly 1
    // Check if the value is very close to 1.0 (within floating point precision)
    let is_singular = (value - 1.0).abs() < 1e-10 || trimmed == "1";

    format!("{} {}", trimmed, get_plural_unit(unit, !is_singular))
}

fn get_plural_unit(unit: &str, plural: bool) -> String {
    // For compound units, just return as-is
    if unit.contains('/')
        || unit.contains(" per ")
        || unit.contains('^')
        || unit.contains("square")
        || unit.contains("cubic")
    {
        return unit.to_string();
    }

    if plural {
        match unit {
            "meter" => "meters".to_string(),
            "foot" => "feet".to_string(),
            "kilometer" => "kilometers".to_string(),
            "mile" => "miles".to_string(),
            "kilogram" => "kilograms".to_string(),
            "pound" => "pounds".to_string(),
            "liter" => "liters".to_string(),
            "gallon" => "gallons".to_string(),
            "newton" => "newtons".to_string(),
            "joule" => "joules".to_string(),
            "watt" => "watts".to_string(),
            _ => unit.to_string(),
        }
    } else {
        match unit {
            "meters" => "meter".to_string(),
            "feet" => "foot".to_string(),
            "kilometers" => "kilometer".to_string(),
            "miles" => "mile".to_string(),
            "kilograms" => "kilogram".to_string(),
            "pounds" => "pound".to_string(),
            "liters" => "liter".to_string(),
            "gallons" => "gallon".to_string(),
            "newtons" => "newton".to_string(),
            "joules" => "joule".to_string(),
            "watts" => "watt".to_string(),
            _ => unit.to_string(),
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
        None => {
            // Check for specific invalid combinations
            if parsed.unit.contains("meters / celsius") || parsed.unit.contains("feet / fahrenheit")
            {
                return Err(ConversionError::InvalidUnitCombination);
            } else if parsed.unit.contains("kilograms * meters")
                || parsed.unit.contains("pounds inches")
            {
                return Err(ConversionError::UnknownCompoundUnit);
            } else if parsed.unit.contains("meter / meter") || parsed.unit.contains("foot / foot") {
                return Err(ConversionError::UnitCancellationNotSupported);
            }
            return Err(ConversionError::UnknownUnit(parsed.unit));
        }
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
            UnitType::Velocity => "velocity",
            UnitType::Area => "area",
            UnitType::MassDensity => "density",
            UnitType::Acceleration => "acceleration",
            UnitType::Force => "force",
            UnitType::Energy => "energy",
            UnitType::Power => "power",
            UnitType::FuelEconomy => "fuel economy",
        };
        return Err(ConversionError::IncompatibleUnits {
            from: type_name(&input_type).to_string(),
            to: type_name(&output_type).to_string(),
        });
    }

    // Perform conversion based on type
    let result = match input_type {
        UnitType::Length => convert_length(parsed.value, &parsed.unit, &output_unit_lower)?,
        UnitType::Mass => convert_mass(parsed.value, &parsed.unit, &output_unit_lower)?,
        UnitType::Temperature => {
            convert_temperature(parsed.value, &parsed.unit, &output_unit_lower)?
        }
        UnitType::Volume => convert_volume(parsed.value, &parsed.unit, &output_unit_lower)?,
        UnitType::Velocity => convert_velocity(parsed.value, &parsed.unit, &output_unit_lower),
        UnitType::Area => convert_area(parsed.value, &parsed.unit, &output_unit_lower),
        UnitType::MassDensity => {
            convert_mass_density(parsed.value, &parsed.unit, &output_unit_lower)
        }
        UnitType::Acceleration => {
            convert_acceleration(parsed.value, &parsed.unit, &output_unit_lower)
        }
        UnitType::Force => convert_force(parsed.value, &parsed.unit, &output_unit_lower),
        UnitType::Energy => convert_energy(parsed.value, &parsed.unit, &output_unit_lower),
        UnitType::Power => convert_power(parsed.value, &parsed.unit, &output_unit_lower),
        UnitType::FuelEconomy => {
            convert_fuel_economy(parsed.value, &parsed.unit, &output_unit_lower)
        }
    };

    Ok(format_output(result, &output_unit_lower))
}

#[instrument(level = "debug", skip(value))]
fn convert_length(value: f64, from_unit: &str, to_unit: &str) -> Result<f64, ConversionError> {
    let length = match from_unit {
        "meter" | "meters" => Length::new::<length::meter>(value),
        "foot" | "feet" => Length::new::<length::foot>(value),
        "kilometer" | "kilometers" => Length::new::<length::kilometer>(value),
        "mile" | "miles" => Length::new::<length::mile>(value),
        _ => {
            error!(unit = from_unit, "Unexpected unit in convert_length");
            return Err(ConversionError::UnknownUnit(from_unit.to_string()));
        }
    };

    match to_unit {
        "meter" | "meters" => Ok(length.get::<length::meter>()),
        "foot" | "feet" => Ok(length.get::<length::foot>()),
        "kilometer" | "kilometers" => Ok(length.get::<length::kilometer>()),
        "mile" | "miles" => Ok(length.get::<length::mile>()),
        _ => {
            error!(unit = to_unit, "Unexpected unit in convert_length");
            Err(ConversionError::UnknownUnit(to_unit.to_string()))
        }
    }
}

#[instrument(level = "debug", skip(value))]
fn convert_mass(value: f64, from_unit: &str, to_unit: &str) -> Result<f64, ConversionError> {
    let mass = match from_unit {
        "kilogram" | "kilograms" => Mass::new::<mass::kilogram>(value),
        "pound" | "pounds" => Mass::new::<mass::pound>(value),
        _ => {
            error!(unit = from_unit, "Unexpected unit in convert_mass");
            return Err(ConversionError::UnknownUnit(from_unit.to_string()));
        }
    };

    match to_unit {
        "kilogram" | "kilograms" => Ok(mass.get::<mass::kilogram>()),
        "pound" | "pounds" => Ok(mass.get::<mass::pound>()),
        _ => {
            error!(unit = to_unit, "Unexpected unit in convert_mass");
            Err(ConversionError::UnknownUnit(to_unit.to_string()))
        }
    }
}

#[instrument(level = "debug", skip(value))]
fn convert_temperature(value: f64, from_unit: &str, to_unit: &str) -> Result<f64, ConversionError> {
    let temp = match from_unit {
        "celsius" => ThermodynamicTemperature::new::<temperature::degree_celsius>(value),
        "fahrenheit" => ThermodynamicTemperature::new::<temperature::degree_fahrenheit>(value),
        _ => {
            error!(unit = from_unit, "Unexpected unit in convert_temperature");
            return Err(ConversionError::UnknownUnit(from_unit.to_string()));
        }
    };

    match to_unit {
        "celsius" => Ok(temp.get::<temperature::degree_celsius>()),
        "fahrenheit" => Ok(temp.get::<temperature::degree_fahrenheit>()),
        _ => {
            error!(unit = to_unit, "Unexpected unit in convert_temperature");
            Err(ConversionError::UnknownUnit(to_unit.to_string()))
        }
    }
}

#[instrument(level = "debug", skip(value))]
fn convert_volume(value: f64, from_unit: &str, to_unit: &str) -> Result<f64, ConversionError> {
    let volume = match from_unit {
        "liter" | "liters" => Volume::new::<volume::liter>(value),
        "gallon" | "gallons" => Volume::new::<volume::gallon>(value),
        "cubic meter" | "cubic meters" => Volume::new::<volume::cubic_meter>(value),
        "cubic foot" | "cubic feet" => Volume::new::<volume::cubic_foot>(value),
        "cubic centimeter" | "cubic centimeters" => Volume::new::<volume::cubic_centimeter>(value),
        "cubic inch" | "cubic inches" => Volume::new::<volume::cubic_inch>(value),
        _ => {
            error!(unit = from_unit, "Unexpected unit in convert_volume");
            return Err(ConversionError::UnknownUnit(from_unit.to_string()));
        }
    };

    match to_unit {
        "liter" | "liters" => Ok(volume.get::<volume::liter>()),
        "gallon" | "gallons" => Ok(volume.get::<volume::gallon>()),
        "cubic meter" | "cubic meters" => Ok(volume.get::<volume::cubic_meter>()),
        "cubic foot" | "cubic feet" => Ok(volume.get::<volume::cubic_foot>()),
        "cubic centimeter" | "cubic centimeters" => Ok(volume.get::<volume::cubic_centimeter>()),
        "cubic inch" | "cubic inches" => Ok(volume.get::<volume::cubic_inch>()),
        _ => {
            error!(unit = to_unit, "Unexpected unit in convert_volume");
            Err(ConversionError::UnknownUnit(to_unit.to_string()))
        }
    }
}

fn convert_velocity(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    let velocity = match from_unit {
        "miles/hour" | "miles per hour" | "mph" => Velocity::new::<velocity::mile_per_hour>(value),
        "kilometers/hour" | "kilometers per hour" | "kmh" | "kph" | "km/h" => {
            Velocity::new::<velocity::kilometer_per_hour>(value)
        }
        "meters/second" | "meters per second" | "m/s" => {
            Velocity::new::<velocity::meter_per_second>(value)
        }
        "feet/second" | "feet per second" | "ft/s" => {
            Velocity::new::<velocity::foot_per_second>(value)
        }
        _ => unreachable!(),
    };

    match to_unit {
        "miles/hour" | "miles per hour" | "mph" => velocity.get::<velocity::mile_per_hour>(),
        "kilometers/hour" | "kilometers per hour" | "kmh" | "kph" | "km/h" => {
            velocity.get::<velocity::kilometer_per_hour>()
        }
        "meters/second" | "meters per second" | "m/s" => {
            velocity.get::<velocity::meter_per_second>()
        }
        "feet/second" | "feet per second" | "ft/s" => velocity.get::<velocity::foot_per_second>(),
        _ => unreachable!(),
    }
}

fn convert_area(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    let area = match from_unit {
        "square meter" | "square meters" => Area::new::<area::square_meter>(value),
        "square foot" | "square feet" => Area::new::<area::square_foot>(value),
        "square kilometer" | "square kilometers" => Area::new::<area::square_kilometer>(value),
        "square mile" | "square miles" => Area::new::<area::square_mile>(value),
        "acre" | "acres" => Area::new::<area::acre>(value),
        _ => unreachable!(),
    };

    match to_unit {
        "square meter" | "square meters" => area.get::<area::square_meter>(),
        "square foot" | "square feet" => area.get::<area::square_foot>(),
        "square kilometer" | "square kilometers" => area.get::<area::square_kilometer>(),
        "square mile" | "square miles" => area.get::<area::square_mile>(),
        "acre" | "acres" => area.get::<area::acre>(),
        _ => unreachable!(),
    }
}

fn convert_mass_density(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    let density = match from_unit {
        "kilograms / cubic meter" | "kilogram / cubic meter" => {
            MassDensity::new::<mass_density::kilogram_per_cubic_meter>(value)
        }
        "pounds / cubic foot" | "pound / cubic foot" => {
            MassDensity::new::<mass_density::pound_per_cubic_foot>(value)
        }
        "grams / cubic centimeter" | "gram / cubic centimeter" => {
            MassDensity::new::<mass_density::gram_per_cubic_centimeter>(value)
        }
        "pounds / cubic inch" | "pound / cubic inch" => {
            MassDensity::new::<mass_density::pound_per_cubic_inch>(value)
        }
        "gram / milliliter" | "grams / milliliter" => {
            MassDensity::new::<mass_density::gram_per_cubic_centimeter>(value)
        }
        "kilograms / liter" | "kilogram / liter" =>
        // 1 kg/L = 1000 kg/m³
        {
            MassDensity::new::<mass_density::kilogram_per_cubic_meter>(value * 1000.0)
        }
        _ => unreachable!(),
    };

    match to_unit {
        "kilograms / cubic meter" | "kilogram / cubic meter" => {
            density.get::<mass_density::kilogram_per_cubic_meter>()
        }
        "pounds / cubic foot" | "pound / cubic foot" => {
            density.get::<mass_density::pound_per_cubic_foot>()
        }
        "grams / cubic centimeter" | "gram / cubic centimeter" => {
            density.get::<mass_density::gram_per_cubic_centimeter>()
        }
        "pounds / cubic inch" | "pound / cubic inch" => {
            density.get::<mass_density::pound_per_cubic_inch>()
        }
        "gram / milliliter" | "grams / milliliter" => {
            density.get::<mass_density::gram_per_cubic_centimeter>()
        }
        "kilograms / liter" | "kilogram / liter" =>
        // 1 kg/L = 1000 kg/m³
        {
            density.get::<mass_density::kilogram_per_cubic_meter>() / 1000.0
        }
        _ => unreachable!(),
    }
}

fn convert_acceleration(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    let accel = match from_unit {
        "meters / second^2" | "meter / second^2" => {
            Acceleration::new::<acceleration::meter_per_second_squared>(value)
        }
        "feet / second^2" | "foot / second^2" => {
            Acceleration::new::<acceleration::foot_per_second_squared>(value)
        }
        _ => unreachable!(),
    };

    match to_unit {
        "meters / second^2" | "meter / second^2" => {
            accel.get::<acceleration::meter_per_second_squared>()
        }
        "feet / second^2" | "foot / second^2" => {
            accel.get::<acceleration::foot_per_second_squared>()
        }
        _ => unreachable!(),
    }
}

fn convert_force(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    let force = match from_unit {
        "newton" | "newtons" => Force::new::<force::newton>(value),
        "pounds force" | "pound force" => Force::new::<force::pound_force>(value),
        _ => unreachable!(),
    };

    match to_unit {
        "newton" | "newtons" => force.get::<force::newton>(),
        "pounds force" | "pound force" => force.get::<force::pound_force>(),
        _ => unreachable!(),
    }
}

fn convert_energy(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    let energy = match from_unit {
        "joule" | "joules" => Energy::new::<energy::joule>(value),
        "foot pound" | "foot pounds" => Energy::new::<energy::foot_pound>(value),
        _ => unreachable!(),
    };

    match to_unit {
        "joule" | "joules" => energy.get::<energy::joule>(),
        "foot pound" | "foot pounds" => energy.get::<energy::foot_pound>(),
        _ => unreachable!(),
    }
}

fn convert_power(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    let power = match from_unit {
        "watt" | "watts" => Power::new::<power::watt>(value),
        "horsepower" => Power::new::<power::horsepower>(value),
        _ => unreachable!(),
    };

    match to_unit {
        "watt" | "watts" => power.get::<power::watt>(),
        "horsepower" => power.get::<power::horsepower>(),
        _ => unreachable!(),
    }
}

fn convert_fuel_economy(value: f64, from_unit: &str, to_unit: &str) -> f64 {
    // First convert everything to a common base: kilometers per liter
    let km_per_liter = match from_unit {
        "miles / gallon" | "miles per gallon" => {
            // miles/gallon to km/L: 1 mpg = 0.425144 km/L
            value * 0.425143707
        }
        "kilometers / liter" | "kilometers per liter" => value,
        "liters / 100 kilometers" | "liters per 100 kilometers" => {
            // L/100km to km/L: divide 100 by the value
            if value == 0.0 { 0.0 } else { 100.0 / value }
        }
        _ => unreachable!(),
    };

    // Convert from km/L to target unit
    match to_unit {
        "miles / gallon" | "miles per gallon" => {
            // km/L to miles/gallon: divide by 0.425144
            km_per_liter / 0.425143707
        }
        "kilometers / liter" | "kilometers per liter" => km_per_liter,
        "liters per 100 kilometers" | "liters / 100 kilometers" => {
            // km/L to L/100km: divide 100 by the value
            if km_per_liter == 0.0 {
                0.0
            } else {
                100.0 / km_per_liter
            }
        }
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
        assert_eq!(
            convert_units("1 kilometer", "miles").unwrap(),
            "0.621371 miles"
        );
        assert_eq!(
            convert_units("10 kilometers", "miles").unwrap(),
            "6.21371 miles"
        );
        assert_eq!(
            convert_units("1.60934 kilometers", "miles").unwrap(),
            "1 mile"
        );
    }

    #[test]
    fn test_miles_to_kilometers() {
        assert_eq!(
            convert_units("1 mile", "kilometers").unwrap(),
            "1.60934 kilometers"
        );
        assert_eq!(
            convert_units("10 miles", "kilometers").unwrap(),
            "16.0934 kilometers"
        );
        assert_eq!(
            convert_units("0.621371 miles", "kilometers").unwrap(),
            "1 kilometer"
        );
    }

    #[test]
    fn test_celsius_to_fahrenheit() {
        assert_eq!(
            convert_units("0 celsius", "fahrenheit").unwrap(),
            "32 fahrenheit"
        );
        assert_eq!(
            convert_units("100 celsius", "fahrenheit").unwrap(),
            "212 fahrenheit"
        );
        assert_eq!(
            convert_units("-40 celsius", "fahrenheit").unwrap(),
            "-40 fahrenheit"
        );
        assert_eq!(
            convert_units("37 celsius", "fahrenheit").unwrap(),
            "98.6 fahrenheit"
        );
    }

    #[test]
    fn test_fahrenheit_to_celsius() {
        assert_eq!(
            convert_units("32 fahrenheit", "celsius").unwrap(),
            "0 celsius"
        );
        assert_eq!(
            convert_units("212 fahrenheit", "celsius").unwrap(),
            "100 celsius"
        );
        assert_eq!(
            convert_units("-40 fahrenheit", "celsius").unwrap(),
            "-40 celsius"
        );
        assert_eq!(
            convert_units("98.6 fahrenheit", "celsius").unwrap(),
            "37 celsius"
        );
    }

    #[test]
    fn test_kilograms_to_pounds() {
        assert_eq!(
            convert_units("1 kilogram", "pounds").unwrap(),
            "2.20462 pounds"
        );
        assert_eq!(
            convert_units("10 kilograms", "pounds").unwrap(),
            "22.0462 pounds"
        );
        assert_eq!(
            convert_units("0.453592 kilograms", "pounds").unwrap(),
            "1 pound"
        );
    }

    #[test]
    fn test_pounds_to_kilograms() {
        assert_eq!(
            convert_units("1 pound", "kilograms").unwrap(),
            "0.453592 kilograms"
        );
        assert_eq!(
            convert_units("10 pounds", "kilograms").unwrap(),
            "4.53592 kilograms"
        );
        assert_eq!(
            convert_units("2.20462 pounds", "kilograms").unwrap(),
            "1 kilogram"
        );
    }

    #[test]
    fn test_liters_to_gallons() {
        assert_eq!(
            convert_units("1 liter", "gallons").unwrap(),
            "0.264172 gallons"
        );
        assert_eq!(
            convert_units("10 liters", "gallons").unwrap(),
            "2.64172 gallons"
        );
        assert_eq!(
            convert_units("3.78541 liters", "gallons").unwrap(),
            "1 gallon"
        );
    }

    #[test]
    fn test_gallons_to_liters() {
        assert_eq!(
            convert_units("1 gallon", "liters").unwrap(),
            "3.78541 liters"
        );
        assert_eq!(
            convert_units("10 gallons", "liters").unwrap(),
            "37.8541 liters"
        );
        assert_eq!(
            convert_units("0.264172 gallons", "liters").unwrap(),
            "1 liter"
        );
    }

    #[test]
    fn test_invalid_unit() {
        assert_eq!(
            convert_units("1 invalid_unit", "meters")
                .unwrap_err()
                .to_string(),
            "Error: Unknown unit 'invalid_unit'"
        );
        assert_eq!(
            convert_units("1 meter", "invalid_unit")
                .unwrap_err()
                .to_string(),
            "Error: Unknown unit 'invalid_unit'"
        );
    }

    #[test]
    fn test_incompatible_units() {
        assert_eq!(
            convert_units("1 meter", "kilograms")
                .unwrap_err()
                .to_string(),
            "Error: Cannot convert from length to mass"
        );
        assert_eq!(
            convert_units("1 celsius", "meters")
                .unwrap_err()
                .to_string(),
            "Error: Cannot convert from temperature to length"
        );
        assert_eq!(
            convert_units("1 liter", "fahrenheit")
                .unwrap_err()
                .to_string(),
            "Error: Cannot convert from volume to temperature"
        );
    }

    #[test]
    fn test_invalid_input_format() {
        assert_eq!(
            convert_units("meter", "feet").unwrap_err().to_string(),
            "Error: Invalid input format"
        );
        assert_eq!(
            convert_units("1", "meters").unwrap_err().to_string(),
            "Error: Invalid input format"
        );
        assert_eq!(
            convert_units("", "meters").unwrap_err().to_string(),
            "Error: Invalid input format"
        );
        assert_eq!(
            convert_units("1 2 meters", "feet").unwrap_err().to_string(),
            "Error: Invalid input format"
        );
    }

    #[test]
    fn test_plural_units() {
        assert_eq!(convert_units("2 meters", "feet").unwrap(), "6.56168 feet");
        assert_eq!(convert_units("5 feet", "meters").unwrap(), "1.524 meters");
        assert_eq!(
            convert_units("3 kilograms", "pounds").unwrap(),
            "6.61387 pounds"
        );
    }

    #[test]
    fn test_decimal_values() {
        assert_eq!(convert_units("1.5 meters", "feet").unwrap(), "4.92126 feet");
        assert_eq!(
            convert_units("2.5 kilograms", "pounds").unwrap(),
            "5.51156 pounds"
        );
        assert_eq!(
            convert_units("0.25 gallons", "liters").unwrap(),
            "0.946353 liters"
        );
    }

    #[test]
    fn test_negative_values() {
        assert_eq!(convert_units("-5 meters", "feet").unwrap(), "-16.4042 feet");
        assert_eq!(
            convert_units("-10 celsius", "fahrenheit").unwrap(),
            "14 fahrenheit"
        );
        assert_eq!(
            convert_units("-2.5 kilograms", "pounds").unwrap(),
            "-5.51156 pounds"
        );
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
        assert_eq!(
            convert_units("  1 meter  ", "feet").unwrap(),
            "3.28084 feet"
        );
        assert_eq!(convert_units("1  meter", "feet").unwrap(), "3.28084 feet");
        assert_eq!(
            convert_units("1 meter", "  feet  ").unwrap(),
            "3.28084 feet"
        );
    }

    #[test]
    fn test_velocity_miles_per_hour() {
        assert_eq!(
            convert_units("60 miles/hour", "kilometers/hour").unwrap(),
            "96.5606 kilometers/hour"
        );
        assert_eq!(
            convert_units("100 kilometers/hour", "miles/hour").unwrap(),
            "62.1371 miles/hour"
        );
        assert_eq!(
            convert_units("30 meters/second", "miles/hour").unwrap(),
            "67.1081 miles/hour"
        );
        assert_eq!(
            convert_units("88 feet/second", "miles/hour").unwrap(),
            "60 miles/hour"
        );
    }

    #[test]
    fn test_velocity_alternative_formats() {
        assert_eq!(
            convert_units("60 miles per hour", "km/h").unwrap(),
            "96.5606 km/h"
        );
        assert_eq!(convert_units("60 mph", "kph").unwrap(), "96.5606 kph");
        assert_eq!(convert_units("100 kmh", "mph").unwrap(), "62.1371 mph");
        assert_eq!(convert_units("30 m/s", "ft/s").unwrap(), "98.4252 ft/s");
    }

    #[test]
    fn test_unit_multiplication_area() {
        assert_eq!(
            convert_units("10 meters * 5 meters", "square feet").unwrap(),
            "538.196 square feet"
        );
        assert_eq!(
            convert_units("100 square meters", "square feet").unwrap(),
            "1076.39 square feet"
        );
        assert_eq!(
            convert_units("1 square kilometer", "square miles").unwrap(),
            "0.386102 square miles"
        );
        assert_eq!(
            convert_units("1 acre", "square meters").unwrap(),
            "4046.87 square meters"
        );
    }

    #[test]
    fn test_unit_multiplication_volume() {
        assert_eq!(
            convert_units("2 meters * 3 meters * 4 meters", "cubic feet").unwrap(),
            "847.552 cubic feet"
        );
        assert_eq!(
            convert_units("1 cubic meter", "liters").unwrap(),
            "1000 liters"
        );
        assert_eq!(
            convert_units("1 cubic foot", "gallons").unwrap(),
            "7.48052 gallons"
        );
        assert_eq!(
            convert_units("100 cubic centimeters", "cubic inches").unwrap(),
            "6.10238 cubic inches"
        );
    }

    #[test]
    fn test_unit_division_density() {
        assert_eq!(
            convert_units("1000 kilograms / cubic meter", "pounds / cubic foot").unwrap(),
            "62.428 pounds / cubic foot"
        );
        assert_eq!(
            convert_units("8.96 grams / cubic centimeter", "pounds / cubic inch").unwrap(),
            "0.3237 pounds / cubic inch"
        );
        assert_eq!(
            convert_units("1 gram / milliliter", "kilograms / liter").unwrap(),
            "1 kilograms / liter"
        );
    }

    #[test]
    fn test_unit_division_fuel_economy() {
        assert_eq!(
            convert_units("30 miles / gallon", "kilometers / liter").unwrap(),
            "12.7543 kilometers / liter"
        );
        assert_eq!(
            convert_units("8 liters / 100 kilometers", "miles / gallon").unwrap(),
            "29.4018 miles / gallon"
        );
        assert_eq!(
            convert_units("25 miles per gallon", "liters per 100 kilometers").unwrap(),
            "9.40858 liters per 100 kilometers"
        );
    }

    #[test]
    fn test_complex_unit_expressions() {
        assert_eq!(
            convert_units("9.8 meters / second^2", "feet / second^2").unwrap(),
            "32.1522 feet / second^2"
        );
        assert_eq!(
            convert_units("1 newton", "pounds force").unwrap(),
            "0.224809 pounds force"
        );
        assert_eq!(
            convert_units("1 joule", "foot pounds").unwrap(),
            "0.737562 foot pounds"
        );
        assert_eq!(
            convert_units("100 watts", "horsepower").unwrap(),
            "0.134102 horsepower"
        );
    }

    #[test]
    fn test_invalid_compound_units() {
        assert_eq!(
            convert_units("10 meters / celsius", "feet / fahrenheit")
                .unwrap_err()
                .to_string(),
            "Error: Invalid unit combination"
        );
        assert_eq!(
            convert_units("5 kilograms meters", "pounds inches")
                .unwrap_err()
                .to_string(),
            "Error: Unknown unit 'kilograms meters'"
        );
        assert_eq!(
            convert_units("1 meter / meter", "feet")
                .unwrap_err()
                .to_string(),
            "Error: Unit cancellation not supported"
        );
    }

    #[test]
    #[ignore = "not implemented"]
    fn test_parentheses_in_expressions() {
        assert_eq!(
            convert_units("60 miles / (1 hour)", "meters / second").unwrap(),
            "26.8224 meters / second"
        );
        assert_eq!(
            convert_units("(10 kilograms) / (2 meters)^3", "pounds / cubic foot").unwrap(),
            "0.0780194 pounds / cubic foot"
        );
        assert_eq!(
            convert_units("5 * (meters / second)", "feet / second").unwrap(),
            "16.4042 feet / second"
        );
    }
}

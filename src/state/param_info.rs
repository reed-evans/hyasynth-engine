// src/state/param_info.rs
//
// Parameter metadata for UI display and validation.

use std::fmt;

/// Unique identifier for a parameter within a node type.
pub type ParamId = u32;

/// Display curve for parameter UI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayCurve {
    /// Linear mapping
    Linear,
    /// Logarithmic (good for frequency, gain)
    Logarithmic,
    /// Exponential
    Exponential,
    /// Symmetric around zero (good for pan, pitch)
    Symmetric,
}

impl Default for DisplayCurve {
    fn default() -> Self {
        Self::Linear
    }
}

/// Unit type for parameter display.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ParamUnit {
    #[default]
    None,
    /// Hertz (frequency)
    Hz,
    /// Decibels (gain)
    Db,
    /// Percentage (0-100)
    Percent,
    /// Milliseconds
    Ms,
    /// Seconds
    Seconds,
    /// Semitones
    Semitones,
    /// Pan (-1 to +1)
    Pan,
    /// Beats
    Beats,
}

impl fmt::Display for ParamUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamUnit::None => Ok(()),
            ParamUnit::Hz => write!(f, "Hz"),
            ParamUnit::Db => write!(f, "dB"),
            ParamUnit::Percent => write!(f, "%"),
            ParamUnit::Ms => write!(f, "ms"),
            ParamUnit::Seconds => write!(f, "s"),
            ParamUnit::Semitones => write!(f, "st"),
            ParamUnit::Pan => Ok(()),
            ParamUnit::Beats => write!(f, "beats"),
        }
    }
}

/// Metadata describing a parameter.
///
/// Used by the UI to:
/// - Display appropriate controls (knobs, sliders, etc.)
/// - Validate input ranges
/// - Format values for display
#[derive(Debug, Clone)]
pub struct ParamInfo {
    /// Unique ID within the node type
    pub id: ParamId,

    /// Human-readable name
    pub name: String,

    /// Short name for compact displays
    pub short_name: String,

    /// Minimum value
    pub min: f32,

    /// Maximum value
    pub max: f32,

    /// Default value
    pub default: f32,

    /// Unit for display
    pub unit: ParamUnit,

    /// Display curve for UI mapping
    pub curve: DisplayCurve,

    /// Step size for discrete parameters (0 = continuous)
    pub step: f32,
}

impl ParamInfo {
    pub fn new(id: ParamId, name: impl Into<String>) -> Self {
        let name = name.into();
        let short_name = name.chars().take(4).collect();
        Self {
            id,
            name,
            short_name,
            min: 0.0,
            max: 1.0,
            default: 0.0,
            unit: ParamUnit::None,
            curve: DisplayCurve::Linear,
            step: 0.0,
        }
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn default(mut self, value: f32) -> Self {
        self.default = value;
        self
    }

    pub fn unit(mut self, unit: ParamUnit) -> Self {
        self.unit = unit;
        self
    }

    pub fn curve(mut self, curve: DisplayCurve) -> Self {
        self.curve = curve;
        self
    }

    /// Clamp a value to the valid range.
    #[inline]
    pub fn clamp(&self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }

    /// Normalize a value to 0..1 range.
    #[inline]
    pub fn normalize(&self, value: f32) -> f32 {
        (value - self.min) / (self.max - self.min)
    }

    /// Denormalize a 0..1 value to the parameter range.
    #[inline]
    pub fn denormalize(&self, normalized: f32) -> f32 {
        self.min + normalized * (self.max - self.min)
    }

    /// Format a value for display.
    pub fn format(&self, value: f32) -> String {
        let precision = if self.step > 0.0 { 0 } else { 2 };
        if self.unit == ParamUnit::None {
            format!("{:.prec$}", value, prec = precision)
        } else {
            format!("{:.prec$} {}", value, self.unit, prec = precision)
        }
    }
}

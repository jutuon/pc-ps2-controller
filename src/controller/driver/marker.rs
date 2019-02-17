//! Marker types.

pub struct InterruptsEnabled;
pub struct Disabled;

/// Marker trait to notify user about when interrupts
/// should be disabled or not handled.
pub trait InterruptsDisabled {}
pub trait KeyboardDisabled {}
pub trait AuxiliaryDeviceDisabled {}

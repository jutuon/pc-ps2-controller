//! Marker types.

#[derive(Debug)]
pub struct InterruptsEnabled;

#[derive(Debug)]
pub struct Disabled;

/// Marker trait to notify user about when interrupts
/// should be disabled or not handled.
pub trait InterruptsDisabled {}
pub trait KeyboardDisabled {}
pub trait AuxiliaryDeviceDisabled {}

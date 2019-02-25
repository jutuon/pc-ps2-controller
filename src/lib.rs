
//! Driver for PS/2 controller.
//!
//! # Reference material
//! * <http://classiccomputers.info/down/IBM/IBM_AT_5170/IBM_5170_Technical_Reference_6280070_Sep85.pdf>
//!     * PDF page 149
//! * <http://classiccomputers.info/down/IBM_PS2/documents/PS2_Hardware_Interface_Technical_Reference_May88.pdf>
//!     * PDF page 332
//! * <https://wiki.osdev.org/%228042%22_PS/2_Controller>

#![no_std]
#![forbid(missing_debug_implementations)]

pub mod device;
pub mod controller;


pub use pc_keyboard;

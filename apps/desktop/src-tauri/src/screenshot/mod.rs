//! Screen capture.
//!
//! `xcap` is only referenced inside `capture`, so replacing it stays a one-file
//! change. That matters more here than elsewhere: its macOS backend builds on
//! `objc2-core-graphics` rather than ScreenCaptureKit, and Apple has been steadily
//! deprecating the older capture APIs.

pub mod capture;
pub mod types;

pub use capture::{capture_monitor, capture_region, capture_window};
pub use types::{CaptureMode, CaptureRegion, MonitorInfo, WindowInfo};

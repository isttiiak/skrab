use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How a screenshot was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "CaptureMode.ts")]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    Fullscreen,
    Window,
    Region,
}

impl CaptureMode {
    /// The string stored in `screenshots.capture_mode` (matches the CHECK constraint).
    pub fn as_str(self) -> &'static str {
        match self {
            CaptureMode::Fullscreen => "fullscreen",
            CaptureMode::Window => "window",
            CaptureMode::Region => "region",
        }
    }

    /// Reads the value back from `screenshots.capture_mode`.
    /// Used by the round-trip test today; the history view needs it next.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "fullscreen" => Some(CaptureMode::Fullscreen),
            "window" => Some(CaptureMode::Window),
            "region" => Some(CaptureMode::Region),
            _ => None,
        }
    }
}

/// A display available for capture.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "MonitorInfo.ts")]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    /// Position in the virtual desktop, in physical pixels.
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
    /// Backing scale factor, so the frontend can map CSS points to pixels.
    pub scale: f32,
}

/// An on-screen window available for capture.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "WindowInfo.ts")]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub app_name: String,
    pub width: u32,
    pub height: u32,
}

/// A rectangle in physical pixels on the virtual desktop.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "CaptureRegion.ts")]
#[serde(rename_all = "camelCase")]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl CaptureRegion {
    /// True when the region has real area.
    ///
    /// A click without a drag produces a zero-size rectangle, and cropping to it
    /// would either panic or write an empty PNG.
    pub fn is_usable(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Clamps the region to the bounds of an image of `bounds_w` x `bounds_h`.
    ///
    /// A drag can start inside the screen and end past its edge, and on a
    /// multi-monitor desktop the origin can legitimately be negative. Cropping
    /// outside the captured buffer would panic, so clamp first and let the caller
    /// reject anything that clamps away to nothing.
    pub fn clamped_to(self, bounds_w: u32, bounds_h: u32) -> Option<(u32, u32, u32, u32)> {
        let left = self.x.max(0) as u32;
        let top = self.y.max(0) as u32;
        if left >= bounds_w || top >= bounds_h {
            return None;
        }

        // Right/bottom are computed from the original origin so a negative x does not
        // shift the far edge along with it.
        let right = (self.x + self.width as i32).clamp(0, bounds_w as i32) as u32;
        let bottom = (self.y + self.height as i32).clamp(0, bounds_h as i32) as u32;

        let width = right.saturating_sub(left);
        let height = bottom.saturating_sub(top);
        (width > 0 && height > 0).then_some((left, top, width, height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_mode_round_trips_through_its_db_string() {
        for mode in [
            CaptureMode::Fullscreen,
            CaptureMode::Window,
            CaptureMode::Region,
        ] {
            assert_eq!(CaptureMode::from_str(mode.as_str()), Some(mode));
        }
        assert_eq!(CaptureMode::from_str("scroll"), None);
    }

    fn region(x: i32, y: i32, width: u32, height: u32) -> CaptureRegion {
        CaptureRegion {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn a_click_without_a_drag_is_not_usable() {
        assert!(!region(10, 10, 0, 0).is_usable());
        assert!(!region(10, 10, 40, 0).is_usable());
        assert!(region(10, 10, 1, 1).is_usable());
    }

    #[test]
    fn a_region_inside_the_screen_is_unchanged() {
        assert_eq!(
            region(10, 20, 100, 50).clamped_to(1920, 1080),
            Some((10, 20, 100, 50))
        );
    }

    #[test]
    fn a_drag_past_the_right_edge_is_trimmed() {
        assert_eq!(
            region(1900, 100, 200, 50).clamped_to(1920, 1080),
            Some((1900, 100, 20, 50))
        );
    }

    #[test]
    fn a_negative_origin_keeps_the_far_edge_in_place() {
        // Dragging leftwards off the screen must not drag the right edge with it.
        assert_eq!(
            region(-50, -20, 200, 100).clamped_to(1920, 1080),
            Some((0, 0, 150, 80))
        );
    }

    #[test]
    fn a_region_entirely_off_screen_is_rejected() {
        assert_eq!(region(3000, 100, 200, 100).clamped_to(1920, 1080), None);
        assert_eq!(region(-500, 0, 100, 100).clamped_to(1920, 1080), None);
    }

    #[test]
    fn a_region_exactly_filling_the_screen_survives() {
        assert_eq!(
            region(0, 0, 1920, 1080).clamped_to(1920, 1080),
            Some((0, 0, 1920, 1080))
        );
    }
}

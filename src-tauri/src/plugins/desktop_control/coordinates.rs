use serde::{Deserialize, Serialize};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

/// Represents an absolute point in the Windows Virtual Screen coordinate space.
/// In multi-monitor systems, x and/or y can be negative (e.g. secondary monitor positioned left or above primary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopCoordinates {
    pub x: i32,
    pub y: i32,
}

/// The bounding rectangle of the entire virtual desktop across all connected monitors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualDesktopBounds {
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: i32,
    pub height: i32,
}

impl VirtualDesktopBounds {
    /// Query Windows system metrics for the current virtual desktop bounds
    pub fn current() -> Self {
        unsafe {
            let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            // Fallback for headless/uninitialized environments
            let (vw, vh) = if vw <= 0 || vh <= 0 {
                (1920, 1080)
            } else {
                (vw, vh)
            };

            Self {
                origin_x: vx,
                origin_y: vy,
                width: vw,
                height: vh,
            }
        }
    }

    /// Check if a desktop coordinate point falls within the virtual screen
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.origin_x
            && x < (self.origin_x + self.width)
            && y >= self.origin_y
            && y < (self.origin_y + self.height)
    }
}

/// Metadata required to accurately project model-observed image coordinates back to Windows desktop coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateTransformContext {
    pub desktop_origin_x: i32,
    pub desktop_origin_y: i32,
    pub scale_x: f64,
    pub scale_y: f64,
    pub image_width: u32,
    pub image_height: u32,
    pub original_width: u32,
    pub original_height: u32,
    pub dpi_scale: f64,
}

impl CoordinateTransformContext {
    pub fn new(
        desktop_origin_x: i32,
        desktop_origin_y: i32,
        scale_x: f64,
        scale_y: f64,
        image_width: u32,
        image_height: u32,
        original_width: u32,
        original_height: u32,
        dpi_scale: f64,
    ) -> Self {
        Self {
            desktop_origin_x,
            desktop_origin_y,
            scale_x: if scale_x <= 0.0 { 1.0 } else { scale_x },
            scale_y: if scale_y <= 0.0 { 1.0 } else { scale_y },
            image_width,
            image_height,
            original_width,
            original_height,
            dpi_scale: if dpi_scale <= 0.0 { 1.0 } else { dpi_scale },
        }
    }
}

/// Convert model-provided coordinates on an image (e.g. from vision model inference)
/// back into true Windows desktop coordinates.
///
/// Handles:
/// 1. Out-of-bounds validation on the image.
/// 2. Aspect-ratio preserving downscaling inversion (`x / scale_x`, `y / scale_y`).
/// 3. Window and crop offsets (`desktop_origin_x`, `desktop_origin_y`).
/// 4. Negative virtual screen coordinates on multi-monitor setups.
/// 5. Windows desktop bounding validation.
pub fn transform_image_to_desktop(
    image_x: f64,
    image_y: f64,
    context: &CoordinateTransformContext,
) -> Result<DesktopCoordinates, String> {
    transform_image_to_desktop_with_bounds(image_x, image_y, context, &VirtualDesktopBounds::current())
}

/// Convert image coordinates back to desktop coordinates given explicit virtual desktop bounds.
pub fn transform_image_to_desktop_with_bounds(
    image_x: f64,
    image_y: f64,
    context: &CoordinateTransformContext,
    bounds: &VirtualDesktopBounds,
) -> Result<DesktopCoordinates, String> {
    // 1. Validate image bounds
    if image_x < 0.0 || image_x > context.image_width as f64 {
        return Err(format!(
            "Image X coordinate ({:.1}) is out of bounds [0, {}]",
            image_x, context.image_width
        ));
    }
    if image_y < 0.0 || image_y > context.image_height as f64 {
        return Err(format!(
            "Image Y coordinate ({:.1}) is out of bounds [0, {}]",
            image_y, context.image_height
        ));
    }

    // 2. Invert image scaling to restore original pixel offset
    let unscaled_x = image_x / context.scale_x;
    let unscaled_y = image_y / context.scale_y;

    // 3. Add desktop origin offset
    let desktop_x = context.desktop_origin_x + (unscaled_x.round() as i32);
    let desktop_y = context.desktop_origin_y + (unscaled_y.round() as i32);

    // 4. Validate against virtual desktop bounds
    if !bounds.contains(desktop_x, desktop_y) {
        return Err(format!(
            "Calculated desktop coordinates ({}, {}) fall outside virtual screen bounds [x: {}, y: {}, w: {}, h: {}]",
            desktop_x, desktop_y, bounds.origin_x, bounds.origin_y, bounds.width, bounds.height
        ));
    }

    Ok(DesktopCoordinates {
        x: desktop_x,
        y: desktop_y,
    })
}

/// Validate that raw desktop coordinates are inside the active virtual desktop
pub fn validate_desktop_coordinates(x: i32, y: i32) -> Result<(), String> {
    let bounds = VirtualDesktopBounds::current();
    if bounds.contains(x, y) {
        Ok(())
    } else {
        Err(format!(
            "Desktop coordinates ({x}, {y}) fall outside virtual screen bounds [origin: ({}, {}), size: {}x{}]",
            bounds.origin_x, bounds.origin_y, bounds.width, bounds.height
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_scaling_roundtrip() {
        // Test 1: Coordinate transformation against custom bounds (simulating 4K 3840x2160 display downscaled to 1440x810)
        let ctx_4k = CoordinateTransformContext::new(
            0,
            0,
            1440.0 / 3840.0,
            810.0 / 2160.0,
            1440,
            810,
            3840,
            2160,
            1.0,
        );
        let bounds_4k = VirtualDesktopBounds {
            origin_x: 0,
            origin_y: 0,
            width: 3840,
            height: 2160,
        };
        let res_4k = transform_image_to_desktop_with_bounds(720.0, 405.0, &ctx_4k, &bounds_4k)
            .expect("Should transform cleanly for 4K bounds");
        assert_eq!(res_4k.x, 1920);
        assert_eq!(res_4k.y, 1080);

        // Test 2: Coordinate transformation against current host display bounds (e.g. 1366x768 or 1920x1080)
        let host_bounds = VirtualDesktopBounds::current();
        let target_x = host_bounds.origin_x + 100;
        let target_y = host_bounds.origin_y + 100;
        let ctx_host = CoordinateTransformContext::new(
            host_bounds.origin_x,
            host_bounds.origin_y,
            0.5,
            0.5,
            (host_bounds.width as f64 * 0.5) as u32,
            (host_bounds.height as f64 * 0.5) as u32,
            host_bounds.width as u32,
            host_bounds.height as u32,
            1.0,
        );
        let res_host = transform_image_to_desktop(50.0, 50.0, &ctx_host)
            .expect("Should transform cleanly on host screen");
        assert_eq!(res_host.x, target_x);
        assert_eq!(res_host.y, target_y);
    }

    #[test]
    fn test_multi_monitor_negative_origin() {
        // Secondary monitor placed to the left: origin x = -1920, y = 0
        // Resized from 1920x1080 to 1280x720 (scale = 1280/1920 = 0.6666667)
        let ctx = CoordinateTransformContext::new(
            -1920,
            0,
            1280.0 / 1920.0,
            720.0 / 1080.0,
            1280,
            720,
            1920,
            1080,
            1.0,
        );
        let dual_monitor_bounds = VirtualDesktopBounds {
            origin_x: -1920,
            origin_y: 0,
            width: 3840,
            height: 1080,
        };

        // Click at image (640, 360) -> maps to (-1920 + 960) = -960, 540
        let res = transform_image_to_desktop_with_bounds(640.0, 360.0, &ctx, &dual_monitor_bounds)
            .expect("Should transform negative coordinates cleanly");
        assert_eq!(res.x, -960);
        assert_eq!(res.y, 540);
    }

    #[test]
    fn test_image_out_of_bounds_rejected() {
        let ctx = CoordinateTransformContext::new(0, 0, 1.0, 1.0, 800, 600, 800, 600, 1.0);
        let err = transform_image_to_desktop(850.0, 300.0, &ctx).unwrap_err();
        assert!(err.contains("out of bounds"));

        let err2 = transform_image_to_desktop(400.0, -10.0, &ctx).unwrap_err();
        assert!(err2.contains("out of bounds"));
    }
}

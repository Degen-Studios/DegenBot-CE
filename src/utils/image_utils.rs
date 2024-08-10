use opencv::{core, imgproc};
use opencv::prelude::*;
use log::debug;


/// Overlays an image on top of a base image, resizing the overlay to fit the base image width.
///
/// # Arguments
/// * `base` - The base image to overlay the overlay image on.
/// * `overlay` - The image to overlay on the base image.
/// * `_previous_result` - An optional previous result image, not used in this implementation.
///
/// # Returns
/// A new image with the overlay applied to the base image, or an error if the operation fails.
pub fn overlay_image(base: &Mat, overlay: &Mat, _previous_result: Option<&Mat>) -> Result<Mat, opencv::Error> {
    debug!("Starting overlay_image function");
    let (base_height, base_width) = (base.rows(), base.cols());
    debug!("Base image size: {}x{}", base_width, base_height);

    let mut bgra_base = Mat::default();
    if base.channels() == 3 {
        imgproc::cvt_color(base, &mut bgra_base, imgproc::COLOR_BGR2BGRA, 0)?;
    } else if base.channels() == 4 {
        bgra_base = base.clone();
    } else {
        return Err(opencv::Error::new(opencv::core::StsUnsupportedFormat, "Unsupported base image format"));
    }

    let overlay_aspect = overlay.cols() as f32 / overlay.rows() as f32;

    // Always scale to base width
    let new_width = base_width;
    let new_height = (new_width as f32 / overlay_aspect) as i32;

    // Calculate y_offset, trimming the bottom of the overlay if necessary
    let y_offset = if new_height > base_height {
        0
    } else {
        base_height - new_height
    };

    let mut resized_overlay = Mat::default();
    imgproc::resize(overlay, &mut resized_overlay, core::Size::new(new_width, new_height), 0.0, 0.0, imgproc::INTER_LINEAR)?;
    debug!("Resized overlay size: {}x{}", resized_overlay.cols(), resized_overlay.rows());

    let mut result = bgra_base.clone();

    // Determine the height to use (either full overlay height or trimmed to base height)
    let height_to_use = std::cmp::min(new_height, base_height);

    for y in 0..height_to_use {
        for x in 0..new_width {
            let overlay_pixel = resized_overlay.at_2d::<core::Vec4b>(y, x)?;
            if overlay_pixel[3] > 0 {
                let alpha = overlay_pixel[3] as f32 / 255.0;
                let base_pixel = result.at_2d_mut::<core::Vec4b>(y + y_offset, x)?;
                for c in 0..3 {
                    base_pixel[c] = ((1.0 - alpha) * base_pixel[c] as f32 + alpha * overlay_pixel[c] as f32) as u8;
                }
                base_pixel[3] = 255;
            }
        }
    }

    Ok(result)
}

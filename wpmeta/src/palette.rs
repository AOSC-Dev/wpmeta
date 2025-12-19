use eyre::{Result, eyre};
use hex_color::HexColor;
use image::DynamicImage;
use image::imageops::FilterType;
use material_color_utilities::dislike_analyzer::fix_if_disliked;
use material_color_utilities::hct::Hct;
use material_color_utilities::score::score_with;
use quantette::{ImageRef, PaletteSize, Pipeline, QuantizeMethod};

const QUANTETTE_PALETTE_SIZE: PaletteSize = PaletteSize::from_u16_clamped(128);

thread_local! {
    static QUANTETTE_PIPELINE: Pipeline = Pipeline::new().palette_size(QUANTETTE_PALETTE_SIZE).ditherer(None).quantize_method(QuantizeMethod::kmeans()).parallel(false);
}

fn hct_to_hex_color(input: Hct) -> HexColor {
    let argb = input.to_int();
    let r = unsafe { u8::try_from((argb >> 16) & 0xFF).unwrap_unchecked() };
    let g = unsafe { u8::try_from((argb >> 8) & 0xFF).unwrap_unchecked() };
    let b = unsafe { u8::try_from(argb & 0xFF).unwrap_unchecked() };
    HexColor::rgb(r, g, b)
}

pub fn extract_colors(image: &DynamicImage) -> Result<(HexColor, HexColor)> {
    // Downscale image to 128x128 max
    let image = image.resize(128, 128, FilterType::Lanczos3).to_rgb8();
    let (palette, palette_count) = QUANTETTE_PIPELINE
        .with(Pipeline::clone)
        .input_image(ImageRef::try_from(&image)?)
        .output_srgb8_palette_and_counts()
        .ok_or(eyre!("Failed to generate palette from image"))?;
    let colors_to_population = palette
        .iter()
        .copied()
        .zip(palette_count.iter().copied())
        .map(|(color, count)| {
            let argb = 0xFF00_0000u32
                | ((color.red as u32) << 16)
                | ((color.green as u32) << 8)
                | (color.blue as u32);
            (argb, u16::try_from(count).unwrap_or(u16::MAX))
        })
        .collect();
    let ranked = score_with(colors_to_population, Some(8), None, Some(true));
    let primary = unsafe { *ranked.first().unwrap_unchecked() };
    let primary_hct = Hct::from_int(primary);
    let accent = ranked
        .iter()
        .copied()
        .skip(1)
        .find(|&c| {
            let hct = Hct::from_int(c);
            let hue_distance = (primary_hct.hue() - hct.hue()).abs() % 360.0;
            let hue_distance = if hue_distance > 180.0 {
                360.0 - hue_distance
            } else {
                hue_distance
            };
            (hue_distance >= 25.0) && (hct.chroma() >= 20.0)
        })
        .unwrap_or(*ranked.get(1).unwrap_or(&primary));
    let accent_hct = Hct::from_int(accent);

    Ok((
        hct_to_hex_color(fix_if_disliked(primary_hct)),
        hct_to_hex_color(fix_if_disliked(accent_hct)),
    ))
}

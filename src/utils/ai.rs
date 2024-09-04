use std::io::Cursor;
use actix_web::web::BytesMut;
use tract_onnx::prelude::*;
use image::{ImageReader};
use tract_onnx::tract_core::ndarray::Axis;
use crate::AiModel;

pub async fn check_safety(model: &AiModel,
                          body: &BytesMut) -> TractResult<bool> {

    let cursor = Cursor::new(body.to_vec());

    let img = ImageReader::new(cursor).with_guessed_format()?.decode()?;

    let rgb_img = img.to_rgb8();

    // let img = image::open("image2.jpg").unwrap().to_rgb8();
    let resized = image::imageops::resize(&rgb_img, 224, 224, image::imageops::FilterType::Lanczos3);

    let resized = resized
        .pixels()
        .flat_map(|p| p.0)
        .map(|v| (v as f32 / 255.0 - 0.5) / 0.5)
        .collect::<Vec<f32>>();

    let input_array = tract_ndarray::Array3::from_shape_vec((224, 224, 3), resized)?;
    let transpose_array = input_array.permuted_axes([2,0,1]);
    let input_tensor = transpose_array.insert_axis(Axis(0)).into_tensor();
    let result = model.run(tvec![TValue::from(input_tensor)])?;

    let output = result[0].to_array_view::<f32>()?;
    let flat_output = output.as_slice().expect("Failed to convert to slice");

    if flat_output[0] > flat_output[1] {
        Ok(true)
    } else {
        Ok(false)
    }
}
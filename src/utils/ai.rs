use tract_onnx::prelude::*;
use image::GenericImageView;
use tract_onnx::tract_core::ndarray::Axis;

async fn check_safety() -> TractResult<bool> {

    let model = onnx()
        .model_for_path("model.onnx")
        .with_input_fact(0, InferenceFact::dt_shape(f32::datum_type(), tvec![1, 3, 224, 224]))?
        .into_optimized()
        .into_runnable();

    let img = image::open("image2.jpg").unwrap().to_rgb8();
    let resized = image::imageops::resize(&img, 224, 224, image::imageops::FilterType::Lanczos3);

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

    let best_index = output.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(idx, _)| idx).unwrap();

    if best_index == 0 {
        return Ok(true)
    } else {
        return Ok(false)
    }
}
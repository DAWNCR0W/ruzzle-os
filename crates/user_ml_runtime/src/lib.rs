#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::vec::Vec;

use user_gpu_service::{GpuDevice, GpuError, Tensor};

/// ML runtime errors.
#[derive(Debug, Clone, PartialEq)]
pub enum MlError {
    InvalidInput,
    Gpu(GpuError),
}

/// Dense layer definition.
#[derive(Debug, Clone, PartialEq)]
pub struct DenseLayer {
    pub weights: Tensor,
    pub bias: Vec<f32>,
}

/// Simple feed-forward model.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub layers: Vec<DenseLayer>,
}

impl Model {
    /// Creates a new model from layers.
    pub fn new(layers: Vec<DenseLayer>) -> Self {
        Self { layers }
    }

    /// Runs inference on a single input vector.
    pub fn infer(&self, device: &GpuDevice, input: &[f32]) -> Result<Vec<f32>, MlError> {
        if input.is_empty() {
            return Err(MlError::InvalidInput);
        }
        let mut activation = Tensor {
            rows: 1,
            cols: input.len(),
            data: input.to_vec(),
        };
        for layer in &self.layers {
            if layer.bias.len() != layer.weights.rows {
                return Err(MlError::InvalidInput);
            }
            let output = device.matmul(&activation, &layer.weights).map_err(MlError::Gpu)?;
            let mut activated = Vec::with_capacity(output.data.len());
            for (idx, value) in output.data.iter().enumerate() {
                let bias = layer.bias[idx % layer.bias.len()];
                activated.push(relu(value + bias));
            }
            activation = Tensor {
                rows: output.rows,
                cols: output.cols,
                data: activated,
            };
        }
        Ok(activation.data)
    }
}

fn relu(value: f32) -> f32 {
    if value > 0.0 {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer() -> DenseLayer {
        DenseLayer {
            weights: Tensor::new(2, 2, vec![1.0, 0.0, 0.0, 1.0]).unwrap(),
            bias: vec![0.5, -1.0],
        }
    }

    #[test]
    fn infer_rejects_empty_input() {
        let model = Model::new(vec![layer()]);
        let gpu = GpuDevice::default();
        assert_eq!(model.infer(&gpu, &[]), Err(MlError::InvalidInput));
    }

    #[test]
    fn infer_rejects_mismatched_bias() {
        let model = Model::new(vec![DenseLayer {
            weights: Tensor::new(2, 2, vec![1.0, 0.0, 0.0, 1.0]).unwrap(),
            bias: vec![0.1],
        }]);
        let gpu = GpuDevice::default();
        assert_eq!(
            model.infer(&gpu, &[1.0, 2.0]),
            Err(MlError::InvalidInput)
        );
    }

    #[test]
    fn infer_propagates_gpu_error() {
        let model = Model::new(vec![DenseLayer {
            weights: Tensor::new(3, 1, vec![1.0, 2.0, 3.0]).unwrap(),
            bias: vec![0.0, 0.0, 0.0],
        }]);
        let gpu = GpuDevice::default();
        assert_eq!(
            model.infer(&gpu, &[1.0, 2.0]),
            Err(MlError::Gpu(GpuError::ShapeMismatch))
        );
    }

    #[test]
    fn infer_runs_relu_activation() {
        let model = Model::new(vec![layer()]);
        let gpu = GpuDevice::default();
        let output = model.infer(&gpu, &[1.0, 2.0]).unwrap();
        assert_eq!(output, vec![1.5, 1.0]);
    }

    #[test]
    fn infer_without_layers_returns_input() {
        let model = Model::new(Vec::new());
        let gpu = GpuDevice::default();
        let output = model.infer(&gpu, &[1.0, -2.0]).unwrap();
        assert_eq!(output, vec![1.0, -2.0]);
    }

    #[test]
    fn relu_zeroes_negative() {
        assert_eq!(relu(-1.0), 0.0);
        assert_eq!(relu(0.5), 0.5);
    }
}

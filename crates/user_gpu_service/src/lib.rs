#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// GPU computation errors.
#[derive(Debug, Clone, PartialEq)]
pub enum GpuError {
    ShapeMismatch,
    EmptyTensor,
}

/// Simple tensor representation.
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f32>,
}

impl Tensor {
    /// Creates a tensor from raw data.
    pub fn new(rows: usize, cols: usize, data: Vec<f32>) -> Result<Self, GpuError> {
        if rows == 0 || cols == 0 || data.is_empty() {
            return Err(GpuError::EmptyTensor);
        }
        if data.len() != rows * cols {
            return Err(GpuError::ShapeMismatch);
        }
        Ok(Self { rows, cols, data })
    }

    /// Builds a zero-filled tensor.
    pub fn zeros(rows: usize, cols: usize) -> Result<Self, GpuError> {
        if rows == 0 || cols == 0 {
            return Err(GpuError::EmptyTensor);
        }
        Ok(Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        })
    }

    /// Formats a tensor as a simple string for debug output.
    pub fn format(&self) -> String {
        let mut out = String::new();
        for r in 0..self.rows {
            for c in 0..self.cols {
                let value = self.data[r * self.cols + c];
                out.push_str(&format!("{value:.2} "));
            }
            out.push('\n');
        }
        out
    }
}

/// Minimal GPU device interface.
#[derive(Debug, Default, Clone)]
pub struct GpuDevice;

impl GpuDevice {
    /// Performs element-wise addition.
    pub fn add(&self, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor, GpuError> {
        if lhs.rows != rhs.rows || lhs.cols != rhs.cols {
            return Err(GpuError::ShapeMismatch);
        }
        let mut data = Vec::with_capacity(lhs.data.len());
        for (a, b) in lhs.data.iter().zip(rhs.data.iter()) {
            data.push(a + b);
        }
        Tensor::new(lhs.rows, lhs.cols, data)
    }

    /// Performs matrix multiplication.
    pub fn matmul(&self, lhs: &Tensor, rhs: &Tensor) -> Result<Tensor, GpuError> {
        if lhs.cols != rhs.rows {
            return Err(GpuError::ShapeMismatch);
        }
        let mut out = Tensor::zeros(lhs.rows, rhs.cols)?;
        for i in 0..lhs.rows {
            for k in 0..lhs.cols {
                let a = lhs.data[i * lhs.cols + k];
                for j in 0..rhs.cols {
                    let idx = i * rhs.cols + j;
                    out.data[idx] += a * rhs.data[k * rhs.cols + j];
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tensor_rejects_invalid_shapes() {
        assert_eq!(Tensor::new(0, 1, vec![]), Err(GpuError::EmptyTensor));
        assert_eq!(
            Tensor::new(2, 2, vec![1.0, 2.0]),
            Err(GpuError::ShapeMismatch)
        );
    }

    #[test]
    fn tensor_rejects_zero_cols() {
        assert_eq!(Tensor::new(1, 0, vec![]), Err(GpuError::EmptyTensor));
    }

    #[test]
    fn tensor_rejects_empty_data() {
        assert_eq!(Tensor::new(1, 1, vec![]), Err(GpuError::EmptyTensor));
    }

    #[test]
    fn zeros_rejects_empty_dimensions() {
        assert_eq!(Tensor::zeros(0, 2), Err(GpuError::EmptyTensor));
        assert_eq!(Tensor::zeros(2, 0), Err(GpuError::EmptyTensor));
    }

    #[test]
    fn add_rejects_mismatch() {
        let a = Tensor::new(1, 2, vec![1.0, 2.0]).unwrap();
        let b = Tensor::new(2, 1, vec![1.0, 2.0]).unwrap();
        let gpu = GpuDevice::default();
        assert_eq!(gpu.add(&a, &b), Err(GpuError::ShapeMismatch));
    }

    #[test]
    fn add_rejects_mismatch_with_cols() {
        let a = Tensor::new(1, 2, vec![1.0, 2.0]).unwrap();
        let b = Tensor::new(1, 3, vec![1.0, 2.0, 3.0]).unwrap();
        let gpu = GpuDevice::default();
        assert_eq!(gpu.add(&a, &b), Err(GpuError::ShapeMismatch));
    }

    #[test]
    fn add_succeeds() {
        let a = Tensor::new(1, 2, vec![1.0, 2.0]).unwrap();
        let b = Tensor::new(1, 2, vec![3.0, 4.0]).unwrap();
        let gpu = GpuDevice::default();
        let out = gpu.add(&a, &b).unwrap();
        assert_eq!(out.data, vec![4.0, 6.0]);
    }

    #[test]
    fn matmul_rejects_mismatch() {
        let a = Tensor::new(1, 3, vec![1.0, 2.0, 3.0]).unwrap();
        let b = Tensor::new(2, 2, vec![1.0, 0.0, 0.0, 1.0]).unwrap();
        let gpu = GpuDevice::default();
        assert_eq!(gpu.matmul(&a, &b), Err(GpuError::ShapeMismatch));
    }

    #[test]
    fn matmul_rejects_empty_output() {
        let lhs = Tensor {
            rows: 0,
            cols: 0,
            data: Vec::new(),
        };
        let rhs = Tensor {
            rows: 0,
            cols: 1,
            data: Vec::new(),
        };
        let gpu = GpuDevice::default();
        assert_eq!(gpu.matmul(&lhs, &rhs), Err(GpuError::EmptyTensor));
    }

    #[test]
    fn matmul_computes_values() {
        let a = Tensor::new(2, 2, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        let b = Tensor::new(2, 2, vec![2.0, 0.0, 1.0, 2.0]).unwrap();
        let gpu = GpuDevice::default();
        let out = gpu.matmul(&a, &b).unwrap();
        assert_eq!(out.data, vec![4.0, 4.0, 10.0, 8.0]);
    }

    #[test]
    fn format_outputs_lines() {
        let tensor = Tensor::new(1, 2, vec![1.0, 2.5]).unwrap();
        let text = tensor.format();
        assert!(text.contains("1.00"));
        assert!(text.contains("2.50"));
    }

    #[test]
    fn format_handles_empty_tensor() {
        let tensor = Tensor {
            rows: 0,
            cols: 0,
            data: Vec::new(),
        };
        assert_eq!(tensor.format(), "");
    }
}

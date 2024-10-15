pub mod bifrost;
pub mod util;

use std::{
    fmt::{self, Display},
    io,
    ops::{Add, Div},
};

use hdf5::{Dataset, H5Type};
use ndarray::{Array1, Array2, Axis};
use num_traits::{real::Real, Bounded, FromPrimitive};
use serde::Serialize;

fn read_and_process_dataset_floats<T>(
    dataset: &Dataset,
    axis: usize,
    nth_sample: usize,
) -> io::Result<()>
where
    T: H5Type
        + Serialize
        + num_traits::identities::Zero
        + FromPrimitive
        + Clone
        + Copy
        + std::ops::Div
        + Bounded
        + PartialOrd
        + Display
        + Real
        + for<'a> std::iter::Sum<&'a T>,
    for<'a> &'a T: Add<T, Output = T>,
    <T as Div>::Output: fmt::Display,
{
    let ndims = dataset.ndim();

    match ndims {
        1 => {
            // Read the dataset into a 1D ndarray
            let data_1dim: Array1<T> = dataset.read_1d()?;
        }
        2 => {
            // Read the dataset into a 2D ndarray
            let data_2dim: Array2<T> = dataset.read_2d()?;
            let data_2dim_folded = data_2dim.fold_axis(Axis(axis), T::zero(), |acc, &x| acc + x);
        }
        _ => {
            todo!("Unsupported dataset dimensionality: {ndims}");
        }
    }
    Ok(())
}

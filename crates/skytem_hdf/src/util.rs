use hdf5::Datatype;

#[derive(Debug, Clone, Copy)]
pub enum NativePrimitive {
    Integer32b,
    Integer64b,
    UnsignedInteger32b,
    UnsignedInteger64b,
    Float32b,
    Float64b,
    Pointer(usize),
}

impl NativePrimitive {
    pub fn from_dtype(dtype: &Datatype) -> Self {
        if dtype.is::<i32>() {
            return Self::Integer32b;
        } else if dtype.is::<i64>() {
            return Self::Integer64b;
        } else if dtype.is::<usize>() {
            return Self::Pointer(size_of::<usize>());
        } else if dtype.is::<u32>() {
            return Self::UnsignedInteger32b;
        } else if dtype.is::<u64>() {
            return Self::UnsignedInteger64b;
        } else if dtype.is::<f32>() {
            return Self::Float32b;
        } else if dtype.is::<f64>() {
            return Self::Float64b;
        } else {
            todo!("Unsupported datatype");
        }
    }
}

impl std::fmt::Display for NativePrimitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativePrimitive::Pointer(size) => write!(f, "{}-bit pointer", size * 8),
            NativePrimitive::Integer32b => write!(f, "32-bit integer"),
            NativePrimitive::Integer64b => write!(f, "64-bit integer"),
            NativePrimitive::UnsignedInteger32b => write!(f, "32-bit unsigned integer"),
            NativePrimitive::UnsignedInteger64b => write!(f, "64-bit unsigned integer"),
            NativePrimitive::Float32b => write!(f, "32-bit float"),
            NativePrimitive::Float64b => write!(f, "64-bit float"),
        }
    }
}

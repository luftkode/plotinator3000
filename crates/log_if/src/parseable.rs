use std::io;

pub trait Parseable: Sized {
    /// Create a instance from a reader and return the instance along with the number of bytes read
    fn from_reader(reader: &mut impl io::Read) -> io::Result<(Self, usize)>;
}

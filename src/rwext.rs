use std::io::{self, Read, Write};

use byteorder::{LE, ReadBytesExt, WriteBytesExt};

pub trait ReadVecExt: Read {
    fn read_vec_u8(&mut self, vec_size: usize) -> io::Result<Vec<u8>> {
        if vec_size == 0 {
            return Ok(Vec::new());
        }
        let mut v = vec![0u8; vec_size];
        self.read_exact(&mut v)?;
        Ok(v)
    }

    #[allow(dead_code)]
    fn read_vec(&mut self, vec_size: usize, element_size: usize) -> io::Result<Vec<Vec<u8>>> {
        let mut v = Vec::with_capacity(vec_size);
        for _ in 0..vec_size {
            let mut element = Vec::with_capacity(element_size);
            self.read_exact(&mut element)?;
            v.push(element);
        }
        Ok(v)
    }

    /// Read a vector of `T`s from the stream.
    ///
    /// # Safety
    ///
    /// Using [std::mem::transmute] to convert types.
    unsafe fn read_vec_t_sized<T>(&mut self, vec_size: usize) -> io::Result<Vec<T>>
    where
        T: Sized,
    {
        let mut v = Vec::with_capacity(vec_size);
        for _ in 0..vec_size {
            unsafe {
                let mut element = std::mem::MaybeUninit::<T>::uninit();
                self.read_exact(std::slice::from_raw_parts_mut(
                    element.as_mut_ptr() as *mut u8,
                    std::mem::size_of::<T>(),
                ))?;
                v.push(element.assume_init());
            }
        }
        Ok(v)
    }

    /// Read a vector of `T`s from the stream.
    /// It will read the size of the vector first, then read the elements.
    /// The size is a u32 in little-endian format.
    ///
    /// # Safety
    ///
    /// Using [std::mem::transmute] to convert types.
    #[allow(dead_code)]
    unsafe fn read_vec_t<T>(&mut self) -> io::Result<Vec<T>>
    where
        T: Sized,
    {
        let size = self.read_u32::<LE>()?;
        unsafe { self.read_vec_t_sized(size as usize) }
    }

    fn read_vec_fn<F, T, E>(&mut self, vec_size: usize, mut f: F) -> std::result::Result<Vec<T>, E>
    where
        F: FnMut(&mut Self) -> std::result::Result<T, E>,
    {
        let mut v = Vec::with_capacity(vec_size);
        for _ in 0..vec_size {
            v.push(f(self)?);
        }
        Ok(v)
    }
}

impl<T> ReadVecExt for T where T: Read {}

#[allow(dead_code)]
pub trait WriteVecExt: Write {
    fn write_vec(&mut self, elements: &[impl AsRef<[u8]>]) -> io::Result<usize> {
        let mut size = 0;
        self.write_u32::<LE>(elements.len() as u32)?;
        size += size_of::<u32>();
        for element in elements {
            self.write_all(element.as_ref())?;
            size += element.as_ref().len();
        }
        Ok(size)
    }

    unsafe fn write_vec_t_unsafe<T>(&mut self, elements: &[T]) -> io::Result<usize>
    where
        T: Sized,
    {
        let mut size = 0;
        self.write_u32::<LE>(elements.len() as u32)?;
        size += size_of::<u32>();
        for element in elements {
            unsafe {
                self.write_all(std::slice::from_raw_parts(
                    element as *const T as *const u8,
                    std::mem::size_of::<T>(),
                ))?;
                size += std::mem::size_of::<T>();
            }
        }
        Ok(size)
    }
}

impl<T> WriteVecExt for T where T: Write {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinrwNullString(pub binrw::NullString);

impl serde::Serialize for BinrwNullString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> serde::de::Deserialize<'de> for BinrwNullString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(BinrwNullString(binrw::NullString::from(s)))
    }
}

impl AsRef<binrw::NullString> for BinrwNullString {
    fn as_ref(&self) -> &binrw::NullString {
        &self.0
    }
}

impl binrw::BinRead for BinrwNullString {
    type Args<'a> = ();

    fn read_options<R: Read + io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        Ok(BinrwNullString(binrw::NullString::read_options(
            reader, endian, args,
        )?))
    }
}

impl binrw::BinWrite for BinrwNullString {
    type Args<'a> = ();

    fn write_options<W: Write + io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<()> {
        self.0.write_options(writer, endian, args)?;
        Ok(())
    }
}

use byteorder::ReadBytesExt;
use std::collections::HashMap;
use std::io::{Error, Read, Seek, SeekFrom};

mod block;
mod header;

pub use block::*;
pub use header::*;

#[derive(Debug, Clone)]
pub struct Store {
    pub header: Header,
    //pub block0: Block0,
    //pub blocks: Vec<Block>,
    pub properties: HashMap<u32, Property>,
    pub categories: HashMap<u32, Category>,
}

impl Store {
    pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Self, Error> {
        let header = Header::read_from(reader)?;

        let property_blocks: Vec<PropertyBlock> =
            Self::parse_blocks(reader, header.property_index)?;
        let properties: PropertyBlock = property_blocks.into_iter().collect();

        let category_blocks: Vec<CategoryBlock> =
            Self::parse_blocks(reader, header.category_index)?;
        let categories: CategoryBlock = category_blocks.into_iter().collect();

        let item_kind_index_blocks: Vec<CategoryBlock> =
            Self::parse_blocks(reader, header.item_kind_index)?;
        let item_kind_index: CategoryBlock =
            item_kind_index_blocks.into_iter().collect();

        Ok(Store {
            header,
            properties: properties.data,
            categories: categories.data,
        })
    }

    fn parse_blocks<T: Block>(
        reader: &mut (impl Read + Seek),
        offset: u32,
    ) -> Result<Vec<T>, Error> {
        let property_block_metas =
            Meta::read_chain(reader, Some(T::BLOCK_TYPE), offset)?;

        reader.seek(SeekFrom::Start(offset as u64 * 4096))?;
        property_block_metas
            .into_iter()
            .map(|meta| {
                let next_index = meta.next_block_index;
                let block = T::from_meta(reader, meta)?;
                reader.seek(SeekFrom::Start(next_index as u64 * 4096))?;
                Ok(block)
            })
            .collect()
    }
}

pub fn read_var_int(reader: &mut impl Read) -> Result<(u64, usize), Error> {
    let num = reader.read_u8()?;
    let mut read = 1;

    // Count leading ones by bitwise-negating the
    // number and counting leading zeros
    let extra = (!num).leading_zeros();

    if extra == 0 {
        return Ok((num as u64, read));
    }

    let mut result = (1..=extra)
        .map(|x| (extra - x) * 8)
        .map(|shift| {
            reader.read_u8().map(|b| {
                read += 1;

                (b as u64) << (shift as u64)
            })
        })
        .sum::<Result<u64, _>>()?;

    // Use lower nibble?
    if let 1..=4 = extra {
        // Remove n = extra leading ones by shifting
        let additional = (num << extra) >> extra;

        result += (additional as u64) << (extra * 8);
    }

    Ok((result, read))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn smoke() {
        let mut cursor =
            Cursor::new([0x80, 0xA1, 0x20, 0x83, 0x00, 0x00, 0x00, 0x84]);
        assert_eq!(read_var_int(&mut cursor).unwrap(), (161, 2));
    }

    #[test]
    fn extra8() {
        let mut cursor =
            Cursor::new([0xfe, 0x05, 0x7a, 0xe9, 0x7f, 0xbe, 0x8b, 0x6c]);
        assert_eq!(read_var_int(&mut cursor).unwrap(), (1542518172715884, 8));
    }
}

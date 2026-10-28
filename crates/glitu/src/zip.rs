//! An uncompressed zip archive, the container of Office files (.pptx): its entries are already
//! compressed images (PNG) and short XML.

#[derive(Default)]
pub struct Zip {
    data: Vec<u8>,
    central: Vec<u8>,
    count: u16,
}

impl Zip {
    pub fn add(&mut self, name: &str, bytes: Vec<u8>) {
        let (crc, size, offset) = (crc32(&bytes), bytes.len() as u32, self.data.len() as u32);
        // version 2.0, no flag, stored, dated 1 January 1980
        let common = |v: &mut Vec<u8>| {
            for x in [20u16, 0, 0, 0, 0x21] { v.extend(x.to_le_bytes()) }
            for x in [crc, size, size] { v.extend(x.to_le_bytes()) }
            v.extend((name.len() as u16).to_le_bytes());
            v.extend(0u16.to_le_bytes());
        };
        self.data.extend(0x04034b50u32.to_le_bytes());
        common(&mut self.data);
        self.data.extend(name.as_bytes());
        self.data.extend(bytes);

        self.central.extend(0x02014b50u32.to_le_bytes());
        self.central.extend(20u16.to_le_bytes());
        common(&mut self.central);
        for x in [0u16, 0, 0] { self.central.extend(x.to_le_bytes()) }
        for x in [0u32, offset] { self.central.extend(x.to_le_bytes()) }
        self.central.extend(name.as_bytes());
        self.count += 1;
    }

    /// Appends the entries of another archive
    pub fn append(&mut self, other: Zip) {
        let shift = self.data.len() as u32;
        let mut central = other.central;
        let mut at = 0;
        while at < central.len() {
            let off = at + 42;
            let v = u32::from_le_bytes([central[off], central[off + 1], central[off + 2], central[off + 3]]) + shift;
            central[off..off + 4].copy_from_slice(&v.to_le_bytes());
            let name_len = u16::from_le_bytes([central[at + 28], central[at + 29]]) as usize;
            at += 46 + name_len;
        }
        self.data.extend(other.data);
        self.central.extend(central);
        self.count += other.count;
    }

    pub fn finish(mut self) -> Vec<u8> {
        let (offset, size) = (self.data.len() as u32, self.central.len() as u32);
        self.data.append(&mut self.central);
        self.data.extend(0x06054b50u32.to_le_bytes());
        for x in [0u16, 0, self.count, self.count] { self.data.extend(x.to_le_bytes()) }
        for x in [size, offset] { self.data.extend(x.to_le_bytes()) }
        self.data.extend(0u16.to_le_bytes());
        self.data
    }
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB88320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_reference_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
    }

    #[test]
    fn appended_entries_keep_their_offsets() {
        let mut a = Zip::default();
        a.add("a.txt", b"first".to_vec());
        let mut b = Zip::default();
        b.add("b.txt", b"second".to_vec());
        a.append(b);
        let zip = a.finish();
        // the central directory of b.txt points past a.txt's local entry (30 + name + data)
        let central = zip.windows(4).rposition(|w| w == 0x02014b50u32.to_le_bytes()).unwrap();
        let offset = u32::from_le_bytes([zip[central + 42], zip[central + 43], zip[central + 44], zip[central + 45]]);
        assert_eq!(offset as usize, 30 + "a.txt".len() + "first".len());
        // end of central directory: two entries
        assert_eq!(&zip[zip.len() - 22..zip.len() - 18], 0x06054b50u32.to_le_bytes());
        assert_eq!(zip[zip.len() - 12], 2);
    }
}

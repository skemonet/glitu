//! An uncompressed zip archive, the container of Office files (.pptx): its entries are already
//! compressed images (PNG) and short XML. And the reading of an archive (stored or deflated
//! entries), to unpack the browser glitu downloads.

use std::path::Path;

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

fn u16_at(b: &[u8], at: usize) -> Result<usize, String> {
    b.get(at..at + 2).map(|x| u16::from_le_bytes([x[0], x[1]]) as usize).ok_or_else(|| "truncated zip archive".into())
}

fn u32_at(b: &[u8], at: usize) -> Result<usize, String> {
    b.get(at..at + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize).ok_or_else(|| "truncated zip archive".into())
}

/// Unpacks an archive into `dest`: files, folders, and on Unix the permissions and symbolic links.
/// A name that would leave `dest` is refused. Returns the number of files
pub fn unpack(data: &[u8], dest: &Path) -> Result<usize, String> {
    let end = (0..data.len().saturating_sub(21))
        .rev()
        .find(|&i| data[i..i + 4] == 0x06054b50u32.to_le_bytes())
        .ok_or("not a zip archive")?;
    let (count, mut at) = (u16_at(data, end + 10)?, u32_at(data, end + 16)?);
    let mut files = 0;
    for _ in 0..count {
        if u32_at(data, at)? != 0x02014b50 {
            return Err("damaged zip archive".into());
        }
        let (method, size, full) = (u16_at(data, at + 10)?, u32_at(data, at + 20)?, u32_at(data, at + 24)?);
        let (name_len, extra_len, comment_len) = (u16_at(data, at + 28)?, u16_at(data, at + 30)?, u16_at(data, at + 32)?);
        let (mode, local) = (u32_at(data, at + 38)? >> 16, u32_at(data, at + 42)?);
        let name = String::from_utf8_lossy(data.get(at + 46..at + 46 + name_len).ok_or("truncated zip archive")?).into_owned();
        at += 46 + name_len + extra_len + comment_len;
        if name.starts_with('/') || name.contains('\\') || name.split('/').any(|part| part == "..") {
            return Err(format!("{name}: outside the folder, refused"));
        }
        let target = dest.join(&name);
        if name.ends_with('/') {
            std::fs::create_dir_all(&target).map_err(|e| format!("{}: {e}", target.display()))?;
            continue;
        }
        let start = local + 30 + u16_at(data, local + 26)? + u16_at(data, local + 28)?;
        let raw = data.get(start..start + size).ok_or("truncated zip archive")?;
        let bytes = match method {
            0 => raw.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(raw, full).map_err(|e| format!("{name}: {e:?}"))?,
            m => return Err(format!("{name}: compression {m} not supported")),
        };
        if bytes.len() != full {
            return Err(format!("{name}: damaged entry"));
        }
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        #[cfg(not(unix))]
        let _ = mode;
        #[cfg(unix)]
        if mode & 0o170000 == 0o120000 {
            let link = String::from_utf8_lossy(&bytes).into_owned();
            std::os::unix::fs::symlink(&link, &target).map_err(|e| format!("{}: {e}", target.display()))?;
            continue;
        }
        std::fs::write(&target, &bytes).map_err(|e| format!("{}: {e}", target.display()))?;
        #[cfg(unix)]
        if mode & 0o777 != 0 {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(mode as u32 & 0o777));
        }
        files += 1;
    }
    Ok(files)
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

    #[test]
    fn an_archive_written_is_unpacked() {
        let mut z = Zip::default();
        z.add("a/b.txt", b"hello".to_vec());
        z.add("c.txt", b"world".to_vec());
        let dest = std::env::temp_dir().join(format!("glitu-unzip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dest);
        assert_eq!(unpack(&z.finish(), &dest).unwrap(), 2);
        assert_eq!(std::fs::read(dest.join("a/b.txt")).unwrap(), b"hello");
        let mut bad = Zip::default();
        bad.add("../x", vec![]);
        assert!(unpack(&bad.finish(), &dest).unwrap_err().contains("refused"));
        let _ = std::fs::remove_dir_all(&dest);
        assert!(unpack(b"nope", &dest).is_err());
    }
}

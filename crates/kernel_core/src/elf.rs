use alloc::vec::Vec;

use hal::{Errno, PageFlags};

const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LITTLE: u8 = 1;
const ELF_TYPE_EXEC: u16 = 2;
const ELF_MACHINE_X86_64: u16 = 0x3E;
const PT_LOAD: u32 = 1;

/// Represents a loadable ELF segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadSegment {
    pub vaddr: u64,
    pub mem_size: u64,
    pub file_size: u64,
    pub offset: u64,
    pub flags: PageFlags,
}

/// Parsed ELF image metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedElf {
    pub entry: u64,
    pub segments: Vec<LoadSegment>,
}

/// Trait for mapping and populating ELF segments.
pub trait ElfLoader {
    fn map(&mut self, vaddr: u64, mem_size: u64, flags: PageFlags) -> Result<(), Errno>;
    fn copy(&mut self, vaddr: u64, data: &[u8]) -> Result<(), Errno>;
    fn zero(&mut self, vaddr: u64, size: u64) -> Result<(), Errno>;
}

/// Parses an ELF64 little-endian executable.
pub fn parse_elf(image: &[u8]) -> Result<LoadedElf, Errno> {
    if image.len() < 64 {
        return Err(Errno::InvalidArg);
    }
    if image[..4] != ELF_MAGIC {
        return Err(Errno::InvalidArg);
    }
    if image[4] != ELF_CLASS_64 || image[5] != ELF_DATA_LITTLE {
        return Err(Errno::InvalidArg);
    }
    let e_type = u16::from_le_bytes([image[16], image[17]]);
    let e_machine = u16::from_le_bytes([image[18], image[19]]);
    if e_type != ELF_TYPE_EXEC || e_machine != ELF_MACHINE_X86_64 {
        return Err(Errno::InvalidArg);
    }
    let entry = u64::from_le_bytes([
        image[24],
        image[25],
        image[26],
        image[27],
        image[28],
        image[29],
        image[30],
        image[31],
    ]);
    let phoff = u64::from_le_bytes([
        image[32],
        image[33],
        image[34],
        image[35],
        image[36],
        image[37],
        image[38],
        image[39],
    ]) as usize;
    let phentsize = u16::from_le_bytes([image[54], image[55]]) as usize;
    let phnum = u16::from_le_bytes([image[56], image[57]]) as usize;

    let table_end = phoff.checked_add(phentsize.saturating_mul(phnum)).ok_or(Errno::InvalidArg)?;
    if table_end > image.len() || phentsize < 56 {
        return Err(Errno::InvalidArg);
    }

    let mut segments = Vec::new();
    for index in 0..phnum {
        let base = phoff + index * phentsize;
        let p_type = u32::from_le_bytes([
            image[base],
            image[base + 1],
            image[base + 2],
            image[base + 3],
        ]);
        let p_flags = u32::from_le_bytes([
            image[base + 4],
            image[base + 5],
            image[base + 6],
            image[base + 7],
        ]);
        let p_offset = u64::from_le_bytes([
            image[base + 8],
            image[base + 9],
            image[base + 10],
            image[base + 11],
            image[base + 12],
            image[base + 13],
            image[base + 14],
            image[base + 15],
        ]);
        let p_vaddr = u64::from_le_bytes([
            image[base + 16],
            image[base + 17],
            image[base + 18],
            image[base + 19],
            image[base + 20],
            image[base + 21],
            image[base + 22],
            image[base + 23],
        ]);
        let p_filesz = u64::from_le_bytes([
            image[base + 32],
            image[base + 33],
            image[base + 34],
            image[base + 35],
            image[base + 36],
            image[base + 37],
            image[base + 38],
            image[base + 39],
        ]);
        let p_memsz = u64::from_le_bytes([
            image[base + 40],
            image[base + 41],
            image[base + 42],
            image[base + 43],
            image[base + 44],
            image[base + 45],
            image[base + 46],
            image[base + 47],
        ]);

        if p_type != PT_LOAD {
            continue;
        }
        let data_end = p_offset
            .checked_add(p_filesz)
            .ok_or(Errno::InvalidArg)?;
        if data_end as usize > image.len() || p_memsz < p_filesz {
            return Err(Errno::InvalidArg);
        }

        segments.push(LoadSegment {
            vaddr: p_vaddr,
            mem_size: p_memsz,
            file_size: p_filesz,
            offset: p_offset,
            flags: flags_from_elf(p_flags),
        });
    }

    Ok(LoadedElf { entry, segments })
}

/// Applies loadable segments to the given loader implementation.
pub fn load_elf<L: ElfLoader>(image: &[u8], elf: &LoadedElf, loader: &mut L) -> Result<(), Errno> {
    for segment in &elf.segments {
        loader.map(segment.vaddr, segment.mem_size, segment.flags)?;
        let file_end = segment
            .offset
            .checked_add(segment.file_size)
            .ok_or(Errno::InvalidArg)? as usize;
        let file_start = segment.offset as usize;
        let data = &image[file_start..file_end];
        loader.copy(segment.vaddr, data)?;
        if segment.mem_size > segment.file_size {
            let zero_start = segment.vaddr + segment.file_size;
            let zero_size = segment.mem_size - segment.file_size;
            loader.zero(zero_start, zero_size)?;
        }
    }
    Ok(())
}

fn flags_from_elf(p_flags: u32) -> PageFlags {
    let mut flags = PageFlags::USER;
    if p_flags & 0x4 != 0 {
        flags = flags.union(PageFlags::READ);
    }
    if p_flags & 0x2 != 0 {
        flags = flags.union(PageFlags::WRITE);
    }
    if p_flags & 0x1 != 0 {
        flags = flags.union(PageFlags::EXECUTE);
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestLoader {
        mapped: Vec<(u64, u64, PageFlags)>,
        copied: Vec<(u64, usize)>,
        zeroed: Vec<(u64, u64)>,
        fail_map: bool,
        fail_copy: bool,
        fail_zero: bool,
    }

    impl TestLoader {
        fn new() -> Self {
            Self::default()
        }

        fn fail_map() -> Self {
            Self {
                fail_map: true,
                ..Self::default()
            }
        }

        fn fail_copy() -> Self {
            Self {
                fail_copy: true,
                ..Self::default()
            }
        }

        fn fail_zero() -> Self {
            Self {
                fail_zero: true,
                ..Self::default()
            }
        }
    }

    impl ElfLoader for TestLoader {
        fn map(&mut self, vaddr: u64, mem_size: u64, flags: PageFlags) -> Result<(), Errno> {
            if self.fail_map {
                return Err(Errno::NoMem);
            }
            self.mapped.push((vaddr, mem_size, flags));
            Ok(())
        }

        fn copy(&mut self, vaddr: u64, data: &[u8]) -> Result<(), Errno> {
            if self.fail_copy {
                return Err(Errno::InvalidArg);
            }
            self.copied.push((vaddr, data.len()));
            Ok(())
        }

        fn zero(&mut self, vaddr: u64, size: u64) -> Result<(), Errno> {
            if self.fail_zero {
                return Err(Errno::NoPerm);
            }
            self.zeroed.push((vaddr, size));
            Ok(())
        }
    }

    fn build_test_elf() -> Vec<u8> {
        let mut image = vec![0u8; 0x200];
        image[..4].copy_from_slice(&ELF_MAGIC);
        image[4] = ELF_CLASS_64;
        image[5] = ELF_DATA_LITTLE;
        image[6] = 1;

        image[16..18].copy_from_slice(&ELF_TYPE_EXEC.to_le_bytes());
        image[18..20].copy_from_slice(&ELF_MACHINE_X86_64.to_le_bytes());
        image[20..24].copy_from_slice(&1u32.to_le_bytes());
        image[24..32].copy_from_slice(&0x400000u64.to_le_bytes());
        image[32..40].copy_from_slice(&64u64.to_le_bytes());
        image[52..54].copy_from_slice(&64u16.to_le_bytes());
        image[54..56].copy_from_slice(&56u16.to_le_bytes());
        image[56..58].copy_from_slice(&1u16.to_le_bytes());

        let ph = 64;
        image[ph..ph + 4].copy_from_slice(&PT_LOAD.to_le_bytes());
        image[ph + 4..ph + 8].copy_from_slice(&0x5u32.to_le_bytes());
        image[ph + 8..ph + 16].copy_from_slice(&0x100u64.to_le_bytes());
        image[ph + 16..ph + 24].copy_from_slice(&0x400000u64.to_le_bytes());
        image[ph + 32..ph + 40].copy_from_slice(&4u64.to_le_bytes());
        image[ph + 40..ph + 48].copy_from_slice(&8u64.to_le_bytes());

        image[0x100..0x104].copy_from_slice(&[1, 2, 3, 4]);
        image
    }

    #[test]
    fn parse_valid_elf() {
        let image = build_test_elf();
        let elf = parse_elf(&image).expect("parse should succeed");
        assert_eq!(elf.entry, 0x400000);
        assert_eq!(elf.segments.len(), 1);
        let seg = &elf.segments[0];
        assert_eq!(seg.vaddr, 0x400000);
        assert_eq!(seg.mem_size, 8);
        assert_eq!(seg.file_size, 4);
    }

    #[test]
    fn parse_invalid_magic() {
        let image = vec![0u8; 64];
        let result = parse_elf(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_invalid_header() {
        let mut image = build_test_elf();
        image[4] = 1;
        let result = parse_elf(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_invalid_ph_offset() {
        let mut image = build_test_elf();
        image[32..40].copy_from_slice(&0x400u64.to_le_bytes());
        let result = parse_elf(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_short_image() {
        let image = vec![0u8; 10];
        let result = parse_elf(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_wrong_machine() {
        let mut image = build_test_elf();
        image[18..20].copy_from_slice(&0x28u16.to_le_bytes());
        let result = parse_elf(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_wrong_data_encoding() {
        let mut image = build_test_elf();
        image[5] = 2;
        let result = parse_elf(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_wrong_type() {
        let mut image = build_test_elf();
        image[16..18].copy_from_slice(&1u16.to_le_bytes());
        let result = parse_elf(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_skips_non_load_segments() {
        let mut image = build_test_elf();
        let ph = 64;
        image[ph..ph + 4].copy_from_slice(&0u32.to_le_bytes());
        let elf = parse_elf(&image).expect("parse should succeed");
        assert!(elf.segments.is_empty());
    }

    #[test]
    fn parse_rejects_filesz_larger_than_memsz() {
        let mut image = build_test_elf();
        let ph = 64;
        image[ph + 32..ph + 40].copy_from_slice(&16u64.to_le_bytes());
        image[ph + 40..ph + 48].copy_from_slice(&8u64.to_le_bytes());
        let result = parse_elf(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_small_phentsize() {
        let mut image = build_test_elf();
        image[54..56].copy_from_slice(&40u16.to_le_bytes());
        let result = parse_elf(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_table_overflow() {
        let mut image = build_test_elf();
        image[32..40].copy_from_slice(&u64::MAX.to_le_bytes());
        let result = parse_elf(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_segment_out_of_bounds() {
        let mut image = build_test_elf();
        let ph = 64;
        image[ph + 8..ph + 16].copy_from_slice(&0x1F0u64.to_le_bytes());
        image[ph + 32..ph + 40].copy_from_slice(&32u64.to_le_bytes());
        let result = parse_elf(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn parse_rejects_data_end_overflow() {
        let mut image = build_test_elf();
        let ph = 64;
        image[ph + 8..ph + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        image[ph + 32..ph + 40].copy_from_slice(&1u64.to_le_bytes());
        let result = parse_elf(&image);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn flags_from_elf_sets_all_bits() {
        let flags = flags_from_elf(0x7);
        assert!(flags.contains(PageFlags::READ));
        assert!(flags.contains(PageFlags::WRITE));
        assert!(flags.contains(PageFlags::EXECUTE));
        assert!(flags.contains(PageFlags::USER));
    }

    #[test]
    fn flags_from_elf_defaults_to_user() {
        let flags = flags_from_elf(0x0);
        assert_eq!(flags, PageFlags::USER);
    }


    #[test]
    fn load_elf_invokes_loader() {
        let image = build_test_elf();
        let elf = parse_elf(&image).expect("parse should succeed");
        let mut loader = TestLoader::new();
        load_elf(&image, &elf, &mut loader).expect("load should succeed");

        assert_eq!(loader.mapped.len(), 1);
        assert_eq!(loader.copied.len(), 1);
        assert_eq!(loader.zeroed.len(), 1);
        assert_eq!(loader.zeroed[0], (0x400000 + 4, 4));
    }

    #[test]
    fn load_elf_skips_zero_when_no_bss() {
        let mut image = build_test_elf();
        let ph = 64;
        image[ph + 32..ph + 40].copy_from_slice(&4u64.to_le_bytes());
        image[ph + 40..ph + 48].copy_from_slice(&4u64.to_le_bytes());

        let elf = parse_elf(&image).expect("parse should succeed");
        let mut loader = TestLoader::new();
        load_elf(&image, &elf, &mut loader).expect("load should succeed");
        assert!(loader.zeroed.is_empty());
    }

    #[test]
    fn load_elf_rejects_offset_overflow() {
        let elf = LoadedElf {
            entry: 0,
            segments: vec![LoadSegment {
                vaddr: 0,
                mem_size: 4,
                file_size: 4,
                offset: u64::MAX,
                flags: PageFlags::USER,
            }],
        };
        let image = vec![0u8; 4];
        let mut loader = TestLoader::new();
        let result = load_elf(&image, &elf, &mut loader);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn load_elf_propagates_loader_errors() {
        let image = build_test_elf();
        let elf = parse_elf(&image).expect("parse should succeed");

        let mut loader = TestLoader::fail_map();
        let result = load_elf(&image, &elf, &mut loader);
        assert_eq!(result, Err(Errno::NoMem));
    }

    #[test]
    fn load_elf_propagates_copy_errors() {
        let image = build_test_elf();
        let elf = parse_elf(&image).expect("parse should succeed");

        let mut loader = TestLoader::fail_copy();
        let result = load_elf(&image, &elf, &mut loader);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn load_elf_propagates_zero_errors() {
        let image = build_test_elf();
        let elf = parse_elf(&image).expect("parse should succeed");

        let mut loader = TestLoader::fail_zero();
        let result = load_elf(&image, &elf, &mut loader);
        assert_eq!(result, Err(Errno::NoPerm));
    }

    #[test]
    fn load_elf_succeeds_with_non_failing_loader() {
        let image = build_test_elf();
        let elf = parse_elf(&image).expect("parse should succeed");

        let mut loader = TestLoader::new();
        let result = load_elf(&image, &elf, &mut loader);
        assert_eq!(result, Ok(()));
    }
}

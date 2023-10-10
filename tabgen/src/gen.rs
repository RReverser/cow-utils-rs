//! The basic idea is that we segment codepoints into one of a,
//! few ranges:
//!
//! - Ascii (self explanatory, and handled by the caller).
//! - ChangesUpper (only changes on uppercase).
//! - ChangesLower (only changes on lowercase).
//! - ChangesEither (changes on either upper or lower).
//! - ChangesUpperLowerAlternating (every other character is upper/lower in this
//!   block. May sound weird but is very common, many scripts are layed out with
//!   the equivalent of `A`, `a`, `B`, `b`, etc).
//!
//! Note that the last of these sounds weird but is extremely common, and would
//! otherwise significantly bloat the table.
#![allow(unused)]
const NUM_CHARS: usize = 0x110000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharInfo {
    pub codepoint: u32,
    pub changes_when_upper: bool,
    pub changes_when_lower: bool,
}
impl CharInfo {
    pub fn try_ch(&self) -> Option<char> {
        char::from_u32(self.codepoint)
    }
    pub fn ch(&self) -> char {
        char::from_u32(self.codepoint)
            .unwrap_or_else(|| panic!("0x{:X} is not a valid scalar", self.codepoint))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharCaseChanges {
    Never,
    UpperOnly,
    LowerOnly,
    Always,
}
impl CharCaseChanges {
    // true if we change under exactly one of uppercase/lowercase
    pub fn is_simple_cased(self) -> bool {
        matches!(
            self,
            CharCaseChanges::UpperOnly | CharCaseChanges::LowerOnly
        )
    }
    // true if we change under exactly one of uppercase/lowercase
    pub fn alternates_with(self, o: Self) -> bool {
        match (self, o) {
            (CharCaseChanges::UpperOnly, CharCaseChanges::LowerOnly)
            | (CharCaseChanges::LowerOnly, CharCaseChanges::UpperOnly) => true,
            _ => false,
        }
    }
}

impl CharInfo {
    pub fn case_enum(self) -> CharCaseChanges {
        match (self.changes_when_lower, self.changes_when_upper) {
            (false, false) => CharCaseChanges::Never,
            (false, true) => CharCaseChanges::UpperOnly,
            (true, false) => CharCaseChanges::LowerOnly,
            (true, true) => CharCaseChanges::Always,
        }
    }

    // true if we change under exactly one of uppercase/lowercase
    pub fn alternates_with(&self, o: &Self) -> bool {
        self.case_enum().alternates_with(o.case_enum())
    }
}

pub struct CaseChangeDb {
    pub infos: Box<[CharInfo; NUM_CHARS]>,
}

impl CaseChangeDb {
    pub fn info(&self, c: u32) -> CharInfo {
        self.infos
            .get(c as usize)
            .copied()
            .unwrap_or_else(|| CharInfo {
                codepoint: c,
                changes_when_lower: false,
                changes_when_upper: false,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharRangeType {
    // Uniform(Never) is not present in output
    Uniform(CharCaseChanges),
    AlternatingUpperLower,
    AlternatingLowerUpper,
}
impl CharRangeType {
    pub fn encode(self) -> Option<u32> {
        use CharRangeType::*;
        match self {
            Uniform(CharCaseChanges::UpperOnly) => Some(0),
            Uniform(CharCaseChanges::LowerOnly) => Some(1),
            AlternatingUpperLower => Some(2),
            AlternatingLowerUpper => Some(3),
            Uniform(CharCaseChanges::Always) => Some(4),
            Uniform(CharCaseChanges::Never) => None,
        }
    }
    const ENCNAMES: &'static [&'static str; 5] = &[
        "RK_UNIFORM_UPPER",
        "RK_UNIFORM_LOWER",
        "RK_ALT_UPPER_LOWER",
        "RK_ALT_LOWER_UPPER",
        "RK_UNIFORM_BOTH",
    ];
}
// bottom 8 are len, then 3 kind, top 21 are char
pub const RANGE_ENCODING_LEN_BITS: u32 = 8;
pub const RANGE_ENCODING_KIND_BITS: u32 = 3;
pub const RANGE_ENCODING_CHAR_BITS: u32 = 21;

pub const RANGE_ENCODING_CHAR_SHIFT: u32 = RANGE_ENCODING_LEN_BITS + RANGE_ENCODING_KIND_BITS;
pub const RANGE_ENCODING_KIND_SHIFT: u32 = RANGE_ENCODING_LEN_BITS;

fn encode_direct(kind: u32, char: u32, len: u32) -> u32 {
    debug_assert_eq!(kind & !((1 << RANGE_ENCODING_KIND_BITS) - 1), 0);
    debug_assert_eq!(char & !((1 << RANGE_ENCODING_CHAR_BITS) - 1), 0);
    debug_assert_eq!(len & !((1 << RANGE_ENCODING_LEN_BITS) - 1), 0);
    debug_assert!(len <= RANGE_ENCODING_MAX_LEN as u32);
    debug_assert!(kind <= 4);
    debug_assert!(<char>::from_u32(char).is_some());
    let result = len | (kind << RANGE_ENCODING_KIND_SHIFT) | (char << RANGE_ENCODING_CHAR_SHIFT);
    if cfg!(debug_assertions) {
        let (k, c, l) = decode_direct(result);
        debug_assert!(k == kind && c == char && len == l);
    }
    result
}
fn decode_direct(enc: u32) -> (u32, u32, u32) {
    let len = enc & 0xff;
    let ch = enc >> RANGE_ENCODING_CHAR_SHIFT;
    let kind = (enc >> RANGE_ENCODING_KIND_SHIFT) & ((1 << RANGE_ENCODING_KIND_BITS) - 1);
    (kind, ch, len)
}

// bigger and we split it up
pub const RANGE_ENCODING_MAX_LEN: usize = 254;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoundCharRange {
    pub start_char: u32,
    pub length: usize,
    pub kind: CharRangeType,
}
impl core::fmt::Display for FoundCharRange {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let rk = self
            .kind
            .encode()
            .map(|n| CharRangeType::ENCNAMES[n as usize])
            .unwrap_or("RK_NEVER");
        // use std::fmt::Write;
        // let mut s = String::new();

        if self.length == 1 {
            write!(f, "{rk},len=1 : U+{:04x}", self.start_char)?;
            if let Some(c) = char::from_u32(self.start_char) {
                write!(f, " ({c:?})")?;
            }
        } else if self.length != 0 {
            write!(f, "{rk},len={} : ", self.length)?;
            let end_m1 = self.end() - 1;
            write!(f, "U+{:04x}..=U+{:04x}", self.start_char, end_m1)?;
            if self.length >= 2 {
                write!(f, "U+{:04x}..=U+{:04x}", self.start_char, end_m1)?;
                if let (Some(c), Some(e)) =
                    (char::from_u32(self.start_char), char::from_u32(end_m1))
                {
                    write!(f, " ({:?}..={:?})", c, e)?;
                }
            }
        } else {
            write!(f, "{rk},len=0 : (empty)")?;
        }
        Ok(())
    }
}

impl CharRangeType {
    pub fn is_alt(&self) -> bool {
        matches!(
            self,
            CharRangeType::AlternatingUpperLower | CharRangeType::AlternatingLowerUpper
        )
    }
}

impl FoundCharRange {
    pub fn end(&self) -> u32 {
        self.start_char + (self.length as u32)
    }
    pub fn encode(&self) -> u32 {
        encode_direct(
            self.kind.encode().unwrap(),
            self.start_char,
            self.length as u32,
        )
    }

    pub fn new_uniform_range(db: &CaseChangeDb, c: u32, len: usize) -> Self {
        let kind = db.info(c).case_enum();
        debug_assert!(db.infos[c as usize..][..len]
            .iter()
            .all(|c| c.case_enum() == kind));
        debug_assert!(len != 0);
        Self {
            start_char: c,
            length: len,
            kind: CharRangeType::Uniform(kind),
        }
    }
    pub fn new_alt_range(db: &CaseChangeDb, c: u32, len: usize) -> Self {
        let kind = db.info(c).case_enum();
        let slice = &db.infos[c as usize..][..len];

        // debug_assert!(slice.iter().all(|c| c.case_enum() == kind));
        debug_assert!(len >= 2);
        let expected_seq = {
            let arr = if CharCaseChanges::LowerOnly == kind {
                [CharCaseChanges::LowerOnly, CharCaseChanges::UpperOnly]
            } else {
                [CharCaseChanges::UpperOnly, CharCaseChanges::LowerOnly]
            };
            arr.into_iter().cycle().take(len)
        };
        debug_assert!(
            slice.iter().map(|c| c.case_enum()).eq(expected_seq),
            "{:?}",
            slice.iter().map(|c| c.case_enum()).collect::<Vec<_>>()
        );

        Self {
            start_char: c,
            length: len,
            kind: match kind {
                CharCaseChanges::LowerOnly => CharRangeType::AlternatingLowerUpper,
                CharCaseChanges::UpperOnly => CharRangeType::AlternatingUpperLower,
                CharCaseChanges::Always | CharCaseChanges::Never => unreachable!(),
            },
        }
    }
    pub fn split_into_chunks(&self, db: &CaseChangeDb, lenmax: usize) -> Vec<Self> {
        let mut v = Vec::with_capacity(self.length / lenmax + 1);

        if self.length < lenmax {
            v.push(self.clone());
        } else {
            let mut c0 = self.start_char;
            let end = self.end();
            while c0 < end {
                let mut next_end = c0 + (lenmax as u32);
                if next_end > end {
                    next_end = end;
                }
                let new_len = next_end - c0;

                let new_kind = if self.kind.is_alt() {
                    match db.info(c0).case_enum() {
                        CharCaseChanges::Always | CharCaseChanges::Never => unreachable!(),
                        chty if new_len == 1 => CharRangeType::Uniform(chty),
                        CharCaseChanges::LowerOnly => CharRangeType::AlternatingLowerUpper,
                        CharCaseChanges::UpperOnly => CharRangeType::AlternatingUpperLower,
                    }
                } else {
                    self.kind
                };
                v.push(Self {
                    start_char: c0,
                    length: new_len as usize,
                    kind: new_kind,
                });

                c0 = next_end;
            }
        }

        v
    }
}

// basically `slice.group_by(test).next().map(|s| s.len()).unwrap_or_default()`,
// but group_by is unstable
fn group_len<T, F: Fn(&T, &T) -> bool>(slice: &[T], test: F) -> usize {
    if slice.len() < 2 {
        return slice.len();
    }
    let mut len = 1;
    let mut iter = slice.windows(2);
    while let Some([l, r]) = iter.next() {
        if test(l, r) {
            len += 1;
        } else {
            break;
        }
    }
    len
}

impl CaseChangeDb {
    pub fn new() -> Self {
        let info: Vec<CharInfo> = (0..NUM_CHARS as u32)
            .map(|cp| {
                if let Some(ch) = char::from_u32(cp) {
                    CharInfo {
                        codepoint: cp,
                        changes_when_lower: changes_when_lowercased(ch),
                        changes_when_upper: changes_when_uppercased(ch),
                    }
                } else {
                    CharInfo {
                        codepoint: cp,
                        changes_when_lower: false,
                        changes_when_upper: false,
                    }
                }
            })
            .collect();
        CaseChangeDb {
            infos: info.try_into().unwrap(),
        }
    }
    // simple greedy approach
    pub fn find_ranges(&self) -> Vec<FoundCharRange> {
        let mut ranges = Vec::with_capacity(1000);
        let mut cur = 0;
        while (cur as usize) < self.infos.len() {
            let found = self.find_range_from(cur);
            debug_assert_eq!(found.start_char, cur);
            let end = found.end();
            ranges.push(found);
            cur = end;
        }
        // if !raw {
        ranges.retain_mut(|r| self.should_keep_range(r));
        // }
        ranges
    }

    fn find_range_from(&self, c: u32) -> FoundCharRange {
        let first = self.infos[c as usize].case_enum();
        // let len = self.infos[c as usize..].group_by(|a, b| a.case_enum() ==
        // b.case_enum()).next().unwrap().len();
        let uniform_len = group_len(&self.infos[c as usize..], |a, b| {
            a.case_enum() == b.case_enum()
        });
        if uniform_len == 1 && self.info(c).alternates_with(&self.info(c + 1)) {
            debug_assert!(matches!(
                first,
                CharCaseChanges::UpperOnly | CharCaseChanges::LowerOnly
            ));
            let mut alt_len = group_len(&self.infos[c as usize..], |a, b| a.alternates_with(b));
            if c as usize + alt_len != self.infos.len() {
                // Ensure the first entry in the last pair is counted.
                // alt_len += 1;
            }
            FoundCharRange::new_alt_range(self, c, alt_len)
        } else {
            FoundCharRange::new_uniform_range(self, c, uniform_len)
        }
    }
    pub fn splitify_ranges_for_encoding(
        &self,
        rs: &[FoundCharRange],
    ) -> (Vec<FoundCharRange>, Vec<u32>) {
        let mut split: Vec<FoundCharRange> = vec![];
        for (_i, rng) in rs.iter().enumerate() {
            if rng.length <= RANGE_ENCODING_MAX_LEN {
                split.push(*rng)
            } else {
                let chunks = rng.split_into_chunks(self, RANGE_ENCODING_MAX_LEN);
                assert!(chunks.len() >= 2);
                if !chunks.iter().all(|c| c.length <= RANGE_ENCODING_MAX_LEN) {
                    panic!("{:#?} => {:#?}", rng, chunks);
                }
                split.extend_from_slice(&chunks[..]);
            }
            // splitmap.extend(core::iter::repeat(i).take(chunks.len()));
        }
        let enc = split.iter().map(|r| r.encode()).collect();
        (split, enc)
    }
    pub fn check_encoding(&self, rs: &[FoundCharRange], table: &[u32]) {
        assert_eq!(rs.len(), table.len());
        let mut reported = vec![(false, false); NUM_CHARS];
        for testc in ('\0'..=char::MAX).filter(|c| !c.is_ascii()) {
            let res = self.info(testc as u32);
            let range = check::find_encoded_case_range(testc, table);
            let mut bad = false;
            let want_no_entry = res.case_enum() == CharCaseChanges::Never;
            bad |= range.is_none() != want_no_entry;
            if let Some(rng) = range {
                let (re_kind, re_ch, re_len) = decode_direct(rng);
                debug_assert_eq!(encode_direct(re_kind, re_ch, re_len), rng);
                bad |= !(re_ch..re_ch + re_len).contains(&(testc as u32));
                if let Some(i) = table.iter().position(|c| *c == rng) {
                    let real_range = rs[i];
                    bad |= real_range.start_char != re_ch;
                    bad |= real_range.length != re_len as usize;
                    bad |= real_range.kind.encode() != Some(re_kind);
                } else {
                    bad |= true;
                }
            }
        }
    }

    // pub fn find_ranges(&self) -> Vec<FoundCharRange> {
    //     let mut ranges = Vec::with_capacity(1000);
    //     let mut cur = 0;
    //     while (cur as usize) < self.infos.len() {
    //         let found = self.find_range_from(cur);
    //         debug_assert_eq!(found.start_char, cur);
    //         let end = found.end();
    //         ranges.push(found);
    //     }

    //     let mut cleaned_ranges = Vec::with_capacity(ranges.len());
    //     for range in ranges.iter() {
    //         if !self.should_keep_range(range) {
    //             continue;
    //         }
    //         let chunks = range.split_into_chunks(self, RANGE_ENCODING_MAX_LEN);
    //         cleaned_ranges.extend_from_slice(&chunks);
    //     }
    //     cleaned_ranges
    // }

    pub fn should_keep_range(&self, range: &FoundCharRange) -> bool {
        if range.kind == CharRangeType::Uniform(CharCaseChanges::Never) {
            // empty
            false
        } else if range.start_char < 128 && range.end() < 128 {
            // ASCII
            false
        } else {
            true
        }
    }
}

fn changes_when_lowercased(c: char) -> bool {
    !core::iter::once(c).eq(c.to_lowercase())
}

fn changes_when_uppercased(c: char) -> bool {
    !core::iter::once(c).eq(c.to_uppercase())
}

pub fn emit_tab(name: &str, t: &[u32], r: &[FoundCharRange]) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let mut range_type_stats = [(0usize, 0usize); 5];
    let _ = writeln!(s, "pub(super) const {}: &[u32; {}] = &[\n", name, t.len());
    for (&enc, &dec) in core::iter::zip(t, r) {
        debug_assert!(enc == dec.encode());
        let mut comment = String::new();
        let _ = writeln!(s, "    {enc:#010x}, // {dec}");
        let st = &mut range_type_stats[dec.kind.encode().unwrap() as usize];
        st.0 += 1;
        st.1 += dec.length;
    }
    writeln!(s, "];");
    let mut comm = String::new();
    writeln!(
        comm,
        "// {} ranges / {} bytes. per-rangetype stats:",
        t.len(),
        4 * t.len()
    );
    for (i, (ranges, chars)) in range_type_stats.iter().enumerate() {
        writeln!(
            comm,
            "// - {}, ranges={}, chars={}",
            CharRangeType::ENCNAMES[i],
            ranges,
            chars
        );
    }
    format!("{comm}{s}")
}

type ChangesWhenTableType = [u32];

#[path = "./search.rs"]
mod check;

pub(super) fn changes_when_casemapped_nonascii<const MAP_LOWER: bool>(
    needle: char,
    tab: &super::ChangesWhenTableType,
) -> bool {
    let Some(enc) = find_encoded_case_range(needle, tab) else {
        return false;
    };
    const RK_UNIFORM_UPPER: u32 = 0;
    const RK_UNIFORM_LOWER: u32 = 1;
    const RK_ALT_UPPER_LOWER: u32 = 2;
    const RK_ALT_LOWER_UPPER: u32 = 3;
    const RK_UNIFORM_BOTH: u32 = 4;

    let range_st = enc >> 11;
    let range_len = enc & 0xff;
    let range_kind = (enc >> 8) & 0x7;
    debug_assert!(range_kind <= 4);
    let map_lower = MAP_LOWER;
    let map_upper = !MAP_LOWER;
    match range_kind {
        RK_UNIFORM_BOTH => true,
        RK_UNIFORM_UPPER => map_upper,
        RK_UNIFORM_LOWER => map_lower,
        RK_ALT_UPPER_LOWER | RK_ALT_LOWER_UPPER => {
            let offset = needle as u32 - range_st;
            debug_assert!(offset <= range_len);
            let odd = (offset & 1) != 0;
            let odd_is_lower = range_kind == RK_ALT_UPPER_LOWER;
            if MAP_LOWER {
                odd_is_lower == odd
            } else {
                odd_is_lower == !odd
            }
            // match (range_kind == RK_ALT_UPPER_LOWER, MAP_LOWER) {
            //     (true, true) | (false, true) => !odd,
            //     (true, false) | (false, false) => odd,
            //     _ => false,
            // }
        }
        rk => {
            debug_assert!(false, "bad rangekind {:?}", rk);
            false
        }
    }
}

pub(super) fn find_encoded_case_range(
    needle: char,
    ranges: &super::ChangesWhenTableType,
) -> Option<u32> {
    let pos = ranges.binary_search_by(|&entry| {
        let range_st = entry >> 11;
        let range_len = entry & 0xff;
        if range_st > (needle as u32) {
            core::cmp::Ordering::Greater
        } else if (range_st + range_len) <= (needle as u32) {
            core::cmp::Ordering::Less
        } else {
            core::cmp::Ordering::Equal
        }
    });
    match pos {
        Err(_) => None,
        Ok(n) => Some(ranges[n]),
    }
}

mod search;
mod table;
use table::ChangesWhenTableType;
#[inline]
pub(super) fn changes_when_lowercased(c: char) -> bool {
    if c.is_ascii() {
        c.is_ascii_uppercase()
    } else {
        search::changes_when_casemapped_nonascii::</* lowercase = */ true>(
            c,
            table::CHANGES_WHEN_LOOKUP_TAB,
        )
    }
}
#[inline]
pub(super) fn changes_when_uppercased(c: char) -> bool {
    if c.is_ascii() {
        c.is_ascii_lowercase()
    } else {
        search::changes_when_casemapped_nonascii::</* lowercase = */ false>(
            c,
            table::CHANGES_WHEN_LOOKUP_TAB,
        )
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_paranoia() {
        if core::char::UNICODE_VERSION != super::table::UNICODE_VERSION {
            return;
        }
        for c in '\0'..=char::MAX {
            let lower = changes_when_lowercased_refimpl(c);
            let upper = changes_when_uppercased_refimpl(c);
            let fancy_lower = super::changes_when_lowercased(c);
            let fancy_upper = super::changes_when_uppercased(c);
            assert_eq!(
                (lower, upper),
                (fancy_lower, fancy_upper),
                "wrong for {:?} (U+{:04x})",
                c,
                c as u32
            );
        }
    }

    fn changes_when_lowercased_refimpl(c: char) -> bool {
        !core::iter::once(c).eq(c.to_lowercase())
    }

    fn changes_when_uppercased_refimpl(c: char) -> bool {
        !core::iter::once(c).eq(c.to_uppercase())
    }
}

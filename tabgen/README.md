# Table generator for cow-utils-rs

This generates tables to efficiently answer if a given character will change when uppercased/lowercased with minimal size overhead.

Note that some characters change when uppercased even when they are not lowercase, and vice versa, so `c.is_uppercase()`/`c.is_lowercase()` does not answer this.

What you actually want is to test for the unicode properties `Changes_When_Uppercased` and `Changes_When_Lowercased`. The Rust stdlib doesn't let you query unicode properties directly (especially not obscure ones like that), but you can still answer it using stdlib functionality by performing `c.to_lowercase()`/`c.to_uppercase()`, and seeing if they actually change the character. This straightforward, and only slightly complicated by the fact that those functions return iterators rather than characters (because this conversion may produce multiple characters). That's handled easily enough tho, and you'd end up with:
```rs
fn changes_when_lowercased(c: char) -> bool {
    !core::iter::once(c).eq(c.to_lowercase())
}
fn changes_when_uppercased(c: char) -> bool {
    !core::iter::once(c).eq(c.to_uppercase())
}
```
This works perfectly, but is somewhat slow, which is why this code exists.

First off, ASCII is handled 



// Note: `c.to_uppercase()` and `c.to_lowercase()` return
// an `Iterator<Item = char>`, rather than a character.




The implementation works as follows:
- First, ASCII characters use a small lookup table. The same table is used for both properties.
- 

Then, we binary search for the character to determine





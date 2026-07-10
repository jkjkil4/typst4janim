use std::cell::RefCell;

use typst_kit::fonts::{self, FontStore};

thread_local! {
    static FONTS_CACHE: RefCell<Option<FontStore>> = const { RefCell::new(None) };
}

/// Differs from `typst-cli/src/fonts.rs::discover_fonts`.
/// Here, we don't use `FontArgs` to ignore fonts or include additional fonts;
/// instead, we simply discover all available system fonts using Typst's default behavior.
pub fn discover_all_fonts() -> FontStore {
    let mut fonts = FontStore::new();

    fonts.extend(fonts::system());
    fonts.extend(fonts::embedded());

    fonts
}

/// Borrows the global FontStore cache
///
/// This function cannot be used inside another [with_fonts]
pub fn with_fonts<F, R>(f: F) -> R
where
    F: FnOnce(&FontStore) -> R,
{
    FONTS_CACHE.with(|cell| {
        let mut option = cell.borrow_mut();
        let fonts = option.get_or_insert_with(discover_all_fonts);
        f(fonts)
    })
}

/// Resets the global FontStore cache
///
/// This function cannot be used inside [with_fonts]
pub fn reset_fonts() -> Option<FontStore> {
    FONTS_CACHE.with(|cell| cell.borrow_mut().take())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "already borrowed")]
    fn test_nested_with_fonts_should_panic() {
        with_fonts(|_first_fonts| {
            with_fonts(|_second_fonts| {});
        });
    }

    #[test]
    #[should_panic(expected = "already borrowed")]
    fn test_reset_inside_with_fonts_should_panic() {
        with_fonts(|_| {
            reset_fonts();
        });
    }
}

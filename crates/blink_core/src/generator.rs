use std::fmt;
use std::marker::PhantomData;
use std::num::NonZeroU32;

const GEN_BITS: u32 = 10;
const GEN_MASK: u32 = (1 << GEN_BITS) - 1;
pub const GEN_DEAD: u32 = GEN_MASK;
const MAX_INDEX: usize = (u32::MAX >> GEN_BITS) as usize;

#[repr(transparent)]
pub struct GenId<T> {
    raw: NonZeroU32,
    _marker: PhantomData<fn() -> T>, // Variant-proof marker
}

impl<T> Clone for GenId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for GenId<T> {}
impl<T> PartialEq for GenId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<T> Eq for GenId<T> {}

impl<T> GenId<T> {
    pub fn new(slot: usize, generation: u16) -> Option<Self> {
        let index = slot.checked_add(1)?;
        if index > MAX_INDEX || (generation as u32) >= GEN_DEAD { return None };

        Some(Self {
            raw: NonZeroU32::new(((index as u32) << GEN_BITS) | (generation as u32))?,
            _marker: PhantomData,
        })
    }

    pub fn slot(self) -> usize { (self.raw.get() >> GEN_BITS) as usize - 1 }
    pub fn generation(self) -> u16 { (self.raw.get() & GEN_MASK) as u16 }
}

impl<T> fmt::Debug for GenId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let full_name = std::any::type_name::<T>();
        let short_name = full_name.split("::").last().unwrap_or(full_name);

        f.debug_tuple(short_name)
            .field(&format_args!("{}v{}", self.slot() + 1, self.generation()))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Tag;
    type Id = GenId<Tag>;

    #[test]
    fn round_trip() {
        for &(slot, generation) in &[(0, 0), (1, 1), (12345, 7), (4_194_302, 1022)] {
            let id = Id::new(slot, generation).unwrap();
            assert_eq!(id.slot(), slot);
            assert_eq!(id.generation(), generation);
        }
    }

    #[test]
    fn bounds() {
        assert!(Id::new(MAX_INDEX - 1, 0).is_some()); // largest slot
        assert!(Id::new(MAX_INDEX, 0).is_none());
        assert!(Id::new(usize::MAX, 0).is_none());
        assert!(Id::new(0, GEN_DEAD as u16).is_none()); // dead state is never live
    }

    #[test]
    fn niche_keeps_option_at_four_bytes() {
        assert_eq!(size_of::<Option<Id>>(), 4);
    }

    #[test]
    fn same_slot_different_generation_is_unequal() {
        assert_ne!(Id::new(3, 0).unwrap(), Id::new(3, 1).unwrap());
    }
}

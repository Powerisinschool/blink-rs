use std::fmt;
use std::marker::PhantomData;
use std::num::NonZeroU32;

const GEN_BITS: u32 = 10;
const GEN_MASK: u32 = (1 << GEN_BITS) - 1;
pub const GEN_DEAD: u32 = GEN_MASK;
const MAX_INDEX: usize = (u32::MAX >> GEN_BITS) as usize;

// #[repr(transparent)]
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

        f.debug_tuple(short_name).field(&self.raw).finish()
    }
}

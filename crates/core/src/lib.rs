use std::fmt;
use std::marker::PhantomData;
use std::num::NonZeroU32;

#[repr(transparent)]
pub struct Id<T> {
    raw: NonZeroU32,
    _marker: PhantomData<fn() -> T>, // Variant-proof marker
}

// Implement Clone, Copy, PartialEq manually or via derive
impl<T> Clone for Id<T> { fn clone(&self) -> Self { *self } }
impl<T> Copy for Id<T> {}
impl<T> PartialEq for Id<T> { fn eq(&self, other: &Self) -> bool { self.raw == other.raw } }
impl<T> Eq for Id<T> {}

impl<T> Id<T> {
    pub fn new(id: NonZeroU32) -> Self {
        Self { raw: id, _marker: PhantomData }
    }

    pub fn from_u32(id: u32) -> Option<Self> {
        if id == 0 || id == u32::MAX { return None; }
        NonZeroU32::new(id).map(Self::new)
    }

    pub fn from_usize(id: usize) -> Option<Self> {
        Self::from_u32((id + 1) as u32)
    }

    pub fn get(&self) -> u32 {
        self.raw.get()
    }

    pub fn get_index(&self) -> usize {
        self.raw.get() as usize - 1
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let full_name = std::any::type_name::<T>();
        let short_name = full_name.split("::").last().unwrap_or(full_name);

        f.debug_tuple(short_name)
            .field(&self.raw)
            .finish()
    }
}

use blink_core::base::Id;
use std::fmt;
use std::fmt::Formatter;

#[repr(transparent)]
#[derive(Debug)]
pub struct SlotId<Tag>(Id<Tag>);

impl<T> Clone for SlotId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SlotId<T> {}

impl<Tag> SlotId<Tag> {
    fn from_usize(value: usize) -> Option<Self> {
        // Call the method on the inner Id type, then wrap it in SlotId
        Id::from_usize(value).map(Self)
    }

    pub fn from_slot(slot: usize) -> Option<Self> {
        Self::from_usize(slot)
    }

    pub fn slot(&self) -> usize {
        self.0.get_index()
    }
}

#[derive(Default)]
pub(super) struct SlotStore<T, Tag> {
    entries: Vec<Option<T>>,
    free_slots: Vec<SlotId<Tag>>,
}

impl<T, Tag> SlotStore<T, Tag> {
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            free_slots: Vec::with_capacity(capacity / 4),
        }
    }

    pub(super) fn insert(&mut self, value: impl Into<T>) -> Option<SlotId<Tag>> {
        let value = value.into();

        while let Some(id) = self.free_slots.pop() {
            let Some(entry) = self.entries.get_mut(id.slot()) else {
                continue;
            };

            if entry.is_none() {
                *entry = Some(value);
                return Some(id);
            }
        }

        let id = SlotId::from_slot(self.entries.len())?;
        self.entries.push(Some(value));
        Some(id)
    }

    pub(super) fn get(&self, id: SlotId<Tag>) -> Option<&T> {
        self.entries.get(id.slot())?.as_ref()
    }

    pub(super) fn get_mut(&mut self, id: SlotId<Tag>) -> Option<&mut T> {
        self.entries.get_mut(id.slot())?.as_mut()
    }

    pub(super) fn replace(&mut self, id: SlotId<Tag>, value: impl Into<T>) -> Option<T> {
        let value = value.into();
        self.entries.get_mut(id.slot())?.replace(value)
    }

    pub(super) fn remove(&mut self, id: SlotId<Tag>) -> bool {
        let Some(entry) = self.entries.get_mut(id.slot()) else {
            return false;
        };

        if entry.take().is_none() {
            return false;
        }

        self.free_slots.push(id);
        true
    }
}

impl<T, Tag> fmt::Debug for SlotStore<T, Tag>
where
    T: fmt::Debug,
    Tag: fmt::Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SlotStore")
            .field("entries", &self.entries)
            .field("free_slots", &self.free_slots)
            .finish()
    }
}

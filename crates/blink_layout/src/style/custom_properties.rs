use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU32;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PropId(SymbolU32);

pub struct PropInterner {
    interner: StringInterner<StringBackend>,
}

impl Default for PropInterner {
    fn default() -> Self {
        Self::new()
    }
}

impl PropInterner {
    pub fn new() -> Self {
        PropInterner {
            interner: StringInterner::default(),
        }
    }

    pub fn intern(&mut self, name: &str) -> PropId {
        PropId(self.interner.get_or_intern(name))
    }

    pub fn get(&self, name: &str) -> Option<PropId> {
        self.interner.get(name).map(PropId)
    }

    pub fn get_name(&self, id: PropId) -> Option<&str> {
        self.interner.resolve(id.0)
    }

    pub fn is_empty(&self) -> bool {
        self.interner.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ComputedCustomPropertyValue {
    Invalid,
    Value(Box<str>),
}

pub struct CustomProperty {
    #[allow(unused)]
    id: PropId,
    value: ComputedCustomPropertyValue,
}

impl CustomProperty {
    pub fn value(&self) -> Option<&str> {
        match &self.value {
            ComputedCustomPropertyValue::Invalid => None,
            ComputedCustomPropertyValue::Value(v) => Some(v),
        }
    }
}

pub struct CustomPropTable {
    parent: Vec<u32>,
    props: Vec<Option<Box<[CustomProperty]>>>,
}

impl Default for CustomPropTable {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomPropTable {
    pub fn new() -> Self {
        Self {
            // Initialize the custom property table
            parent: vec![],
            props: vec![],
        }
    }

    #[allow(unused)]
    pub(crate) fn add_stub(&mut self, prop_id: PropId) {
        self.parent.push(0);
        self.props.push(Some(
            vec![CustomProperty {
                id: prop_id,
                value: ComputedCustomPropertyValue::Value("100px".into()),
            }]
            .into_boxed_slice(),
        ));
    }

    // Returns the first occurrence of the custom property in the current node or its ancestors.
    pub fn lookup(&self, _node: usize, _id: PropId) -> Option<&CustomProperty> {
        // TODO: Implement actual lookup logic
        self.props
            .first()
            .and_then(|props| props.as_ref().and_then(|props| props.first()))
    }
}

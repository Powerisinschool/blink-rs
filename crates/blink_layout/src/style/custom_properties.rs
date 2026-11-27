pub struct PropId(u32);

pub enum ComputedCustomPropertyValue {
    Invalid,
    Value(Box<str>),
}

pub struct CustomPropTable {
    parent: Vec<u32>,
    props: Vec<Option<Box<[(PropId, ComputedCustomPropertyValue)]>>>,
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
}

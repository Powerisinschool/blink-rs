use crate::style::matching;
use blink_css::{CSSStyleSheet, ValuePart};
use blink_dom::{Dom, DomNodeId};
use std::collections::HashMap;
use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU32;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomProperty {
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

/// Entries in each block should be sorted by PropId for binary search functionality.
/// Upon review, a linear search often proves faster than a binary search for small arrays.
pub struct CustomPropTable {
    pub(crate) parent: Vec<u32>,
    pub(crate) props: Vec<Option<Box<[CustomProperty]>>>,
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

    pub fn build(
        root: DomNodeId,
        dom: &Dom,
        stylesheet: &CSSStyleSheet,
        interner: &mut PropInterner,
    ) -> Self {
        let nodes_count = dom.nodes_len();
        let mut table = CustomPropTable {
            parent: vec![0; nodes_count],
            props: vec![None; nodes_count],
        };

        let mut stack = vec![(root, 0)];

        while let Some((node_id, parent_slot)) = stack.pop() {
            let slot = node_id.slot();
            table.parent[slot] = parent_slot as u32;

            let mut local_props: HashMap<PropId, &[ValuePart]> = HashMap::new();
            let mut rules = matching::get_matching_rules(node_id, dom, stylesheet);
            rules.sort_by_key(|k| k.specificity());

            // println!("Node {:?} matched rules: {:?}", node_id, rules);

            for rule in rules {
                let Some(declarations) = rule.custom_properties(stylesheet) else {
                    continue;
                };
                for decl in declarations {
                    // println!("Found custom property: {} = {}", decl.name, decl.value);
                    let id = interner.intern(&decl.name);
                    // Later declarations in the cascade overwrite earlier ones.
                    // (Since we're sorting matched `rules` by specificity,
                    // this HashMap insert naturally handles the cascade).
                    local_props.insert(id, decl.value.as_ref());
                }
            }

            if !local_props.is_empty() {
                let mut resolved_props = Vec::with_capacity(local_props.len());
                // outside loop to retain capacity across iterations
                // also, it's given 4 because recursion depth shouldn't exceed that 🙄
                let mut visited: Vec<PropId> = Vec::with_capacity(4);

                for (id, raw_value) in &local_props {
                    visited.push(*id);
                    let resolved = resolve_custom_property_value(
                        raw_value,
                        Some(&local_props),
                        &table,
                        slot,
                        interner,
                        &mut visited,
                    );
                    visited.clear(); // clear `visited`, keeping capacity

                    let final_value = match resolved {
                        Some(value) => ComputedCustomPropertyValue::Value(value.into()),
                        None => ComputedCustomPropertyValue::Invalid,
                    };

                    resolved_props.push(CustomProperty {
                        id: *id,
                        value: final_value,
                    })
                }

                resolved_props.sort_by_key(|p| p.id);
                table.props[slot] = Some(resolved_props.into_boxed_slice());
            }

            let mut child = dom.get_first_child(node_id);
            while let Some(c) = child {
                stack.push((c, slot));
                child = dom.get_next_sibling(c);
            }
        }

        table
    }

    // Returns the first occurrence of the custom property in the current node or its ancestors.
    pub fn lookup(&self, node: usize, id: PropId) -> Option<&str> {
        let mut curr = node;

        loop {
            if let Some(block) = &self.props[curr]
                // && let Ok(pos) = block.binary_search_by_key(&id, |p| p.id)
                && let Some(pos) = block.iter().position(|p| p.id == id)
            {
                return block[pos].value();
            }
            if curr == 0 {
                return None;
            }
            let p = self.parent[curr] as usize;
            debug_assert!(
                p < curr,
                "CustomPropTable invariant violated: parent index {} >= child index {}",
                p,
                curr
            );
            curr = p;
        }
    }
}

/// TODO: Implement `var(--a, fallback)`
/// This will mean refactoring from the current naive approach 😭
fn resolve_custom_property_value(
    raw_value: &[ValuePart],
    local_props: Option<&HashMap<PropId, &[ValuePart]>>,
    table: &CustomPropTable,
    current_slot: usize,
    interner: &PropInterner,
    visited: &mut Vec<PropId>,
) -> Option<String> {
    let mut result = String::with_capacity(raw_value.len());

    for value in raw_value {
        match value {
            ValuePart::Var { name, fallback } => {
                let Some(id) = interner.get(name) else {
                    if let Some(fb) = fallback {
                        result.push_str(&resolve_custom_property_value(
                            fb,
                            local_props,
                            table,
                            current_slot,
                            interner,
                            visited,
                        )?);
                        continue;
                    }
                    return None; // no fallback and unknown variable
                };

                if visited.contains(&id) {
                    return None; // cycle
                }

                visited.push(id);

                let resolved = if let Some(local_val) = local_props.and_then(|p| p.get(&id)) {
                    resolve_custom_property_value(
                        local_val,
                        local_props,
                        table,
                        current_slot,
                        interner,
                        visited,
                    )
                } else if let Some(v) = table.lookup(current_slot, id) {
                    Some(v.to_string())
                } else if let Some(fb) = fallback {
                    resolve_custom_property_value(
                        fb,
                        local_props,
                        table,
                        current_slot,
                        interner,
                        visited,
                    )
                } else {
                    None // no fallback and unknown variable
                };

                visited.pop();
                result.push_str(&resolved?);
            }
            _ => result.push_str(&value.to_string()),
        }
    }

    Some(result)
}

pub fn resolve_var_parts(
    parts: &[ValuePart],
    node_slot: usize,
    table: &CustomPropTable,
    interner: &PropInterner,
) -> Option<String> {
    if !parts.iter().any(|p| matches!(p, ValuePart::Var { .. })) {
        let mut s = String::new();
        for p in parts {
            s.push_str(&p.to_string());
        }
        return Some(s);
    }

    let mut visited = vec![];
    resolve_custom_property_value(parts, None, table, node_slot, interner, &mut visited)
}

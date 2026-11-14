use super::fragment::{FragmentArena, FragmentKind, LayoutFragment, LayoutFragmentId};
use super::inline::layout_inline_run;
use super::space::ConstraintSpace;
use super::text::TextMeasurer;
use crate::geom::{PhysicalOffset, PhysicalRect, PhysicalSize};
use crate::style::{DisplayOutside, compute_style};
use blink_css::CSSStyleSheet;
use blink_dom::{Dom, DomNodeId};

pub struct LayoutContext<'a, M: TextMeasurer + ?Sized> {
    pub dom: &'a Dom,
    pub stylesheet: &'a CSSStyleSheet,
    pub text_measurer: &'a M,
}

impl<'a, M: TextMeasurer + ?Sized> LayoutContext<'a, M> {
    pub fn build_layout_tree(
        &mut self,
        root_id: DomNodeId,
        initial_space: ConstraintSpace,
    ) -> FragmentArena {
        let mut arena = FragmentArena::new();

        let root_fragment_id = self.walk_node(root_id, initial_space, &mut arena);
        arena.set_root(root_fragment_id);

        arena
    }

    pub(crate) fn walk_node(
        &mut self,
        node_id: DomNodeId,
        space: ConstraintSpace,
        arena: &mut FragmentArena,
    ) -> Option<LayoutFragmentId> {
        self.layout_node(node_id, space, arena)
            .map(|fragment| arena.add_fragment(fragment))
    }

    pub(crate) fn layout_node(
        &mut self,
        node_id: DomNodeId,
        space: ConstraintSpace,
        arena: &mut FragmentArena,
    ) -> Option<LayoutFragment> {
        if self
            .dom
            .get_node(node_id)
            .is_some_and(|node| node.is_text())
        {
            return None;
        }

        let computed_style = compute_style(node_id, self.dom, self.stylesheet);

        if computed_style.display.is_none() {
            return None;
        }

        let node_width = computed_style.width;
        let node_height = computed_style.height;

        let is_inline_level = computed_style.display.outside() == Some(DisplayOutside::Inline);

        let initial_width = node_width.unwrap_or(if is_inline_level {
            0.0
        } else {
            space.available_width()
        });

        let available_width = node_width.unwrap_or(space.available_width());
        let child_space = ConstraintSpace::new(available_width, space.available_height());

        let (first_child_id, total_child_height, max_line_width) =
            self.layout_flow_children(node_id, &child_space, arena);

        let node_size = PhysicalSize::new(
            if node_width.is_none() && is_inline_level {
                max_line_width
            } else {
                initial_width
            },
            if node_height.is_none() {
                total_child_height
            } else {
                node_height.unwrap_or_default()
            },
        );
        let bounding_rect = PhysicalRect::new(PhysicalOffset::default(), node_size);
        Some(
            LayoutFragment::new(
                Some(node_id),
                bounding_rect,
                Some(computed_style),
                FragmentKind::Box,
            )
            .with_first_child(first_child_id),
        )
    }

    fn layout_flow_children(
        &mut self,
        node_id: DomNodeId,
        child_space: &ConstraintSpace,
        arena: &mut FragmentArena,
    ) -> (Option<LayoutFragmentId>, f32, f32) {
        let mut current_y_cursor = 0.0;
        let mut max_line_width: f32 = 0.0;
        let mut child_fragments: Vec<LayoutFragment> = Vec::new();

        let mut inline_run: Vec<DomNodeId> = Vec::new();
        let mut current_dom_child = self.dom.get_first_child(node_id);

        while let Some(dom_child) = current_dom_child {
            let child_style = compute_style(dom_child, self.dom, self.stylesheet);

            if child_style.display.is_none() {
                current_dom_child = self.dom.get_next_sibling(dom_child);
                continue;
            }

            let is_inline = self.dom.get_node(dom_child).unwrap().is_text()
                || child_style.display.outside() == Some(DisplayOutside::Inline);

            if is_inline {
                inline_run.push(dom_child);
            } else {
                if !inline_run.is_empty() {
                    let run_width = layout_inline_run(
                        &mut inline_run,
                        child_space.available_width(),
                        &mut current_y_cursor,
                        self,
                        arena,
                        &mut child_fragments,
                    );
                    max_line_width = max_line_width.max(run_width);
                }

                if let Some(block_fragment) = self.layout_node(dom_child, *child_space, arena) {
                    let height = block_fragment.height();
                    let width = block_fragment.width();
                    let new_origin = block_fragment.origin().with_y(current_y_cursor);

                    child_fragments.push(block_fragment.with_origin(new_origin));

                    current_y_cursor += height;
                    max_line_width = max_line_width.max(width);
                }
            }

            current_dom_child = self.dom.get_next_sibling(dom_child);
        }

        if !inline_run.is_empty() {
            let run_width = layout_inline_run(
                &mut inline_run,
                child_space.available_width(),
                &mut current_y_cursor,
                self,
                arena,
                &mut child_fragments,
            );

            max_line_width = max_line_width.max(run_width);
        }

        let mut first_child_fragment_id: Option<LayoutFragmentId> = None;
        for fragment in child_fragments.into_iter().rev() {
            let id = arena.add_fragment(fragment.with_next_sibling(first_child_fragment_id));
            first_child_fragment_id = Some(id);
        }

        (first_child_fragment_id, current_y_cursor, max_line_width)
    }
}

#[cfg(test)]
mod tests {
    use super::super::text::FixedWidth;
    use super::*;
    use blink_css::parse_css_into_stylesheet;
    use blink_dom::get_tag_index;

    fn test_canvas_space() -> ConstraintSpace {
        ConstraintSpace::new(800.0, 600.0)
    }

    #[test]
    fn inline_element_without_explicit_width_shrink_wraps_children() {
        let mut dom = Dom::new();

        let parent = dom.add_element(get_tag_index("div"), None);
        let child = dom.add_element(get_tag_index("span"), None);

        dom.set_parent(child, parent).expect("Failed to set parent");

        dom.set_first_child(parent, Some(child))
            .expect("Failed to set first child");

        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(
            r#"
                div { display: inline-block; }
                span { display: inline-block; width: 40px; height: 10px; }
            "#,
            &mut stylesheet,
        );

        let space = test_canvas_space();

        let mut ctx = LayoutContext {
            dom: &dom,
            stylesheet: &stylesheet,
            text_measurer: &FixedWidth {},
        };

        let arena = ctx.build_layout_tree(parent, space);

        let parent_fragment = arena
            .get_fragment_by_dom_node(parent)
            .expect("Expected parent fragment");

        assert_eq!(
            parent_fragment.width(),
            40.0,
            "Inline element should shrink-wrap its child's width"
        );
    }

    #[test]
    fn test_line_wraps() {
        let mut dom = Dom::new();

        let parent = dom.add_element(get_tag_index("div"), None);

        let child1 = dom.add_element(get_tag_index("span"), None);
        let child2 = dom.add_element(get_tag_index("span"), None);
        let child3 = dom.add_element(get_tag_index("span"), None);

        dom.set_parent(child1, parent)
            .expect("Failed to set parent");
        dom.set_parent(child2, parent)
            .expect("Failed to set parent");
        dom.set_parent(child3, parent)
            .expect("Failed to set parent");

        dom.set_first_child(parent, Some(child1))
            .expect("Failed to set first child");
        dom.set_next_sibling(child1, Some(child2))
            .expect("Failed to set next sibling");
        dom.set_next_sibling(child2, Some(child3))
            .expect("Failed to set next sibling");

        let mut stylesheet = CSSStyleSheet::new();
        parse_css_into_stylesheet(
            r#"
                div { display: block; width: 100px; }
                span { display: inline-block; width: 40px; height: 20px; }
            "#,
            &mut stylesheet,
        );

        let space = test_canvas_space();

        let mut ctx = LayoutContext {
            dom: &dom,
            stylesheet: &stylesheet,
            text_measurer: &FixedWidth {},
        };

        let arena = ctx.build_layout_tree(parent, space);

        let child1_fragment = arena
            .get_fragment_by_dom_node(child1)
            .expect("Expected child1 fragment");

        assert_eq!(
            *child1_fragment.origin(),
            PhysicalOffset::new(0.0, 0.0),
            "Inline element should shrink-wrap its child's width"
        );

        let child2_fragment = arena
            .get_fragment_by_dom_node(child2)
            .expect("Expected child2 fragment");

        assert_eq!(
            *child2_fragment.origin(),
            PhysicalOffset::new(40.0, 0.0),
            "Inline element should shrink-wrap its child's width"
        );

        let child3_fragment = arena
            .get_fragment_by_dom_node(child3)
            .expect("Expected child3 fragment");

        assert_eq!(
            *child3_fragment.origin(),
            PhysicalOffset::new(0.0, 0.0),
            "Inline element should shrink-wrap its child's width"
        );

        let parent_fragment = arena
            .get_fragment_by_dom_node(parent)
            .expect("Expected parent fragment");

        assert_eq!(
            parent_fragment.height(),
            40.0,
            "Inline element should shrink-wrap its child's width"
        );
    }
}

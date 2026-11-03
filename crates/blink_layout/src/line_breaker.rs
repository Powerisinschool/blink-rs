use crate::inline::PreparedInlineItem;

pub struct Line {
    pub items: Vec<PreparedInlineItem>,
    pub width: f32,
    pub height: f32,
}

impl Line {
    pub fn new(items: Vec<PreparedInlineItem>, width: f32, height: f32) -> Self {
        Self {
            items,
            width,
            height,
        }
    }
}

pub fn break_lines(
    items: Vec<PreparedInlineItem>,
    max_width: f32,
    min_line_height: f32,
) -> Vec<Line> {
    let mut completed_lines = Vec::new();
    let mut current_line_items = Vec::new();
    let mut current_line_width = 0.0;
    let mut current_height = 0.0;

    for item in items {
        if item.is_forced_break() {
            break_line(
                min_line_height,
                &mut current_line_items,
                &mut current_line_width,
                &mut current_height,
                &mut completed_lines,
            );
            continue;
        }

        if !current_line_items.is_empty() && current_line_width + item.fit_width > max_width {
            break_line(
                min_line_height,
                &mut current_line_items,
                &mut current_line_width,
                &mut current_height,
                &mut completed_lines,
            );
        }
        current_line_width += item.width;
        current_height = current_height.max(item.height);
        current_line_items.push(item);
    }

    if !current_line_items.is_empty() {
        break_line(
            min_line_height,
            &mut current_line_items,
            &mut current_line_width,
            &mut current_height,
            &mut completed_lines,
        );
    }

    completed_lines
}

fn break_line(
    min_line_height: f32,
    current_line_items: &mut Vec<PreparedInlineItem>,
    current_line_width: &mut f32,
    current_height: &mut f32,
    completed_lines: &mut Vec<Line>,
) {
    let line_width = current_line_items
        .last()
        .map_or(*current_line_width, |item| {
            *current_line_width - (item.width - item.fit_width)
        });
    completed_lines.push(Line::new(
        std::mem::take(current_line_items),
        line_width,
        current_height.max(min_line_height),
    ));
    *current_line_width = 0.0;
    *current_height = 0.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fragment::LayoutFragmentId;
    use crate::inline::PreparedInlineItemKind;
    use blink_dom::{Dom, DomNodeId};

    const LINE_HEIGHT: f32 = 16.0;

    fn test_node() -> (Dom, DomNodeId) {
        let mut dom = Dom::new();
        let node = dom.add_text("");
        (dom, node)
    }

    fn text_item(
        dom_node: DomNodeId,
        text: &str,
        width: f32,
        fit_width: f32,
    ) -> PreparedInlineItem {
        PreparedInlineItem {
            kind: PreparedInlineItemKind::TextSegment {
                dom_node,
                text: text.to_string(),
            },
            width,
            fit_width,
            height: LINE_HEIGHT,
        }
    }

    fn atomic_item(width: f32, fit_width: f32) -> PreparedInlineItem {
        PreparedInlineItem {
            kind: PreparedInlineItemKind::AtomicInline {
                // The line breaker treats the fragment ID as opaque.
                fragment_id: LayoutFragmentId::from_usize(0).unwrap(),
            },
            width,
            fit_width,
            height: 20.0,
        }
    }

    fn forced_break() -> PreparedInlineItem {
        PreparedInlineItem {
            kind: PreparedInlineItemKind::ForcedBreak,
            width: 0.0,
            fit_width: 0.0,
            height: 0.0,
        }
    }

    #[test]
    fn single_text_item_forms_one_line() {
        let (_, node) = test_node();

        let lines = break_lines(vec![text_item(node, "One", 40.0, 30.0)], 100.0, LINE_HEIGHT);

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].items.len(), 1);
        assert_eq!(lines[0].width, 30.0);
        assert_eq!(lines[0].height, LINE_HEIGHT);
    }

    #[test]
    fn text_items_wrap_at_word_boundaries() {
        let (_, node) = test_node();

        let lines = break_lines(
            vec![
                text_item(node, "One", 40.0, 30.0),
                text_item(node, "two", 40.0, 30.0),
                text_item(node, "six", 30.0, 30.0),
            ],
            70.0,
            LINE_HEIGHT,
        );

        assert_eq!(lines.len(), 2);

        // "One two" occupies 70px, excluding the trailing separator.
        assert_eq!(lines[0].items.len(), 2);
        assert_eq!(lines[0].width, 70.0);

        assert_eq!(lines[1].items.len(), 1);
        assert_eq!(lines[1].width, 30.0);
    }

    #[test]
    fn oversized_word_is_placed_on_an_empty_line() {
        let (_, node) = test_node();

        let lines = break_lines(
            vec![text_item(node, "oversized", 90.0, 90.0)],
            50.0,
            LINE_HEIGHT,
        );

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].items.len(), 1);
        assert_eq!(lines[0].width, 90.0);
    }

    #[test]
    fn atomic_inline_item_is_retained_and_can_trigger_wrapping() {
        let (_dom, node) = test_node();

        let lines = break_lines(
            vec![
                // Atomic box: 20px wide, followed by a 10px separator.
                atomic_item(30.0, 20.0),
                text_item(node, "two", 20.0, 10.0),
            ],
            35.0,
            LINE_HEIGHT,
        );

        assert_eq!(lines.len(), 2);

        assert_eq!(lines[0].items.len(), 1);
        assert!(matches!(
            &lines[0].items[0].kind,
            PreparedInlineItemKind::AtomicInline { .. }
        ));
        assert_eq!(lines[0].width, 20.0);

        assert_eq!(lines[1].items.len(), 1);
        assert_eq!(lines[1].width, 10.0);
    }

    #[test]
    fn forced_break_commits_current_line() {
        let (_, node) = test_node();

        let lines = break_lines(
            vec![
                text_item(node, "One", 40.0, 30.0),
                forced_break(),
                text_item(node, "two", 30.0, 30.0),
            ],
            100.0,
            LINE_HEIGHT,
        );

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].items.len(), 1);
        assert_eq!(lines[0].width, 30.0);
        assert_eq!(lines[1].items.len(), 1);
        assert_eq!(lines[1].width, 30.0);
    }

    #[test]
    fn consecutive_forced_breaks_create_empty_line() {
        let (_dom, node) = test_node();

        let lines = break_lines(
            vec![
                text_item(node, "One", 30.0, 30.0),
                forced_break(),
                forced_break(),
                text_item(node, "two", 30.0, 30.0),
            ],
            100.0,
            LINE_HEIGHT,
        );

        assert_eq!(lines.len(), 3);

        assert_eq!(lines[0].items.len(), 1);
        assert_eq!(lines[0].width, 30.0);

        assert!(lines[1].items.is_empty());
        assert_eq!(lines[1].width, 0.0);
        assert_eq!(lines[1].height, LINE_HEIGHT);

        assert_eq!(lines[2].items.len(), 1);
        assert_eq!(lines[2].width, 30.0);
    }

    #[test]
    fn trailing_forced_break_does_not_add_an_extra_line() {
        let (_dom, node) = test_node();

        let lines = break_lines(
            vec![text_item(node, "One", 30.0, 30.0), forced_break()],
            100.0,
            LINE_HEIGHT,
        );

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].items.len(), 1);
        assert_eq!(lines[0].width, 30.0);
    }

    #[test]
    fn empty_input_produces_no_lines() {
        let lines = break_lines(Vec::new(), 100.0, LINE_HEIGHT);

        assert!(lines.is_empty());
    }
}

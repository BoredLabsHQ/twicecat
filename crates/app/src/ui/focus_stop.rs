//! A keyboard stop for a control that iced 0.14 gives none.
//!
//! The iced 0.14 `button` never reports itself to `Operation::focusable`
//! (`iced_widget-0.14.2/src/button.rs`), so Tab skips it. [`focus_stop`]
//! wraps a button: it takes part in Tab and Shift+Tab (`focus_next` and
//! `focus_previous` walk every focusable widget in tree order), draws a
//! focus ring while it holds the focus, and publishes its message on Enter
//! or Space (not while the key is held: a repeat does nothing). Nothing else about the content changes.

use iced::advanced::widget::{Id, Operation, Tree, operation::Focusable, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::keyboard::{self, Key, key::Named};
use iced::{Background, Color, Element, Event, Length, Rectangle, Size, Theme, Vector, border};

/// Width of the focus ring.
const RING: f32 = 2.0;

/// `content` as one Tab stop named `id`, which publishes `message` on Enter
/// or Space while it holds the focus.
pub fn focus_stop<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    id: &'static str,
    message: M,
) -> Element<'a, M> {
    Element::new(FocusStop {
        content: content.into(),
        id: Id::new(id),
        message,
    })
}

#[derive(Debug, Default)]
struct State {
    focused: bool,
}

impl Focusable for State {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
}

struct FocusStop<'a, M> {
    content: Element<'a, M>,
    id: Id,
    message: M,
}

impl<M: Clone> Widget<M, Theme, iced::Renderer> for FocusStop<'_, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        layout::Node::with_children(child.size(), vec![child])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(child) = layout.children().next() else {
            return;
        };
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            child,
            cursor,
            viewport,
        );
        if tree.state.downcast_ref::<State>().focused {
            iced::advanced::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: border::rounded(crate::ui::tokens::S)
                        .color(crate::theme::semantic(theme).focus)
                        .width(RING),
                    ..renderer::Quad::default()
                },
                Background::Color(Color::TRANSPARENT),
            );
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.focusable(
            Some(&self.id),
            layout.bounds(),
            tree.state.downcast_mut::<State>(),
        );
        let Some(child) = layout.children().next() else {
            return;
        };
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], child, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let Some(child) = layout.children().next() else {
            return;
        };
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            child,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        let state = tree.state.downcast_mut::<State>();
        match event {
            // A press anywhere else lets go of the focus, as a text field does.
            Event::Mouse(mouse::Event::ButtonPressed(_))
                if state.focused && !cursor.is_over(layout.bounds()) =>
            {
                state.focused = false;
                shell.request_redraw();
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                repeat: false,
                ..
            }) if state.focused
                && !shell.is_event_captured()
                && !modifiers.control()
                && !modifiers.alt()
                && !modifiers.logo()
                && matches!(key, Key::Named(Named::Enter | Named::Space)) =>
            {
                shell.publish(self.message.clone());
                shell.capture_event();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        layout
            .children()
            .next()
            .map_or(mouse::Interaction::None, |child| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    child,
                    cursor,
                    viewport,
                    renderer,
                )
            })
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, iced::Renderer>> {
        let child = layout.children().next()?;
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            child,
            renderer,
            viewport,
            translation,
        )
    }
}

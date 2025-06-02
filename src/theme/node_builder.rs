//! Tools for creating UI nodes

use bevy::prelude::*;

pub struct NodeBuilder {
    node: Node,
    bg_colour: Option<Color>,
}

impl NodeBuilder {
    pub fn new() -> Self {
        Self {
            node: Node::default(),
            bg_colour: None,
        }
    }

    /// Creates a bundle of UI components for a UI Node
    pub fn build(&self) -> (Node, BackgroundColor) {
        (
            self.node.clone(),
            self.bg_colour
                .map(BackgroundColor)
                .unwrap_or(BackgroundColor::DEFAULT),
        )
    }

    /// Sets the width and height both to the given value
    pub fn sized(mut self, size: Val) -> Self {
        self.node.width = size;
        self.node.height = size;
        self
    }

    /// Sets the width and height to 100%
    pub fn full(mut self) -> Self {
        self.node.width = Val::Percent(100.0);
        self.node.height = Val::Percent(100.0);
        self
    }

    /// sets the background colour of the node
    pub fn background(mut self, colour: impl Into<Color>) -> Self {
        self.bg_colour = Some(colour.into());
        self
    }

    /// flex justifies center and aligns center
    pub fn center_content(mut self) -> Self {
        self.node.justify_content = JustifyContent::Center;
        self.node.align_items = AlignItems::Center;
        self
    }
}

macro_rules! node_method {
    ($name: ident, $type: ty) => {
        node_method!($name, $name, $type);
    };

    ($name: ident, $field: ident, $type: ty) => {
        impl NodeBuilder {
            pub fn $name(mut self, value: $type) -> Self {
                self.node.$field = value;
                self
            }
        }
    };
}

macro_rules! px_method {
    ($name: ident) => {
        impl NodeBuilder {
            pub fn $name(mut self, value: f32) -> Self {
                self.node.$name = Val::Px(value);
                self
            }
        }
    };
}

node_method!(flex_direction, FlexDirection);
node_method!(row_gap, Val);

node_method!(justify, justify_content, JustifyContent);
node_method!(align, align_items, AlignItems);
node_method!(align_content, AlignContent);
node_method!(position, position_type, PositionType);

node_method!(height, Val);
node_method!(width, Val);
node_method!(margin, UiRect);
node_method!(padding, UiRect);

px_method!(left);
px_method!(right);
px_method!(top);
px_method!(bottom);

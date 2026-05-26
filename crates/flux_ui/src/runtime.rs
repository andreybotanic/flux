use std::collections::BTreeSet;

use thiserror::Error;

use crate::types::UiMenuId;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum UiRuntimeError {
    #[error(
        "UiRuntimeError:\n  action: dispatch\n  action_kind: OpenMenu\n  target_menu: {target_menu}\n  reason: target menu not found"
    )]
    OpenMenuTargetMissing { target_menu: Box<str> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiMenuStack {
    stack: Vec<UiMenuId>,
}

impl UiMenuStack {
    pub fn new(initial_menu: UiMenuId) -> Self {
        Self {
            stack: vec![initial_menu],
        }
    }

    #[must_use]
    pub fn current(&self) -> &UiMenuId {
        self.stack
            .last()
            .expect("menu stack always contains at least one menu")
    }

    pub fn push(&mut self, menu_id: UiMenuId) {
        self.stack.push(menu_id);
    }

    #[must_use]
    pub fn back(&mut self) -> bool {
        if self.stack.len() <= 1 {
            return false;
        }
        self.stack.pop();
        true
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinUiActionDispatcher {
    menu_stack: UiMenuStack,
}

impl BuiltinUiActionDispatcher {
    pub fn new(initial_menu: UiMenuId) -> Self {
        Self {
            menu_stack: UiMenuStack::new(initial_menu),
        }
    }

    #[must_use]
    pub fn menu_stack(&self) -> &UiMenuStack {
        &self.menu_stack
    }

    pub fn open_menu(
        &mut self,
        menu_id: &UiMenuId,
        known_menus: &BTreeSet<UiMenuId>,
    ) -> Result<(), UiRuntimeError> {
        if !known_menus.contains(menu_id) {
            return Err(UiRuntimeError::OpenMenuTargetMissing {
                target_menu: menu_id.to_string().into(),
            });
        }
        self.menu_stack.push(menu_id.clone());
        Ok(())
    }

    #[must_use]
    pub fn back_menu(&mut self) -> bool {
        self.menu_stack.back()
    }
}

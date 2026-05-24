use std::collections::BTreeSet;

use thiserror::Error;

use crate::types::{UiAction, UiMenuId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiActionResult {
    Noop,
    MenuChanged,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum UiRuntimeError {
    #[error(
        "UiRuntimeError:\n  action: dispatch\n  action_kind: OpenMenu\n  target_menu: {target_menu}\n  reason: target menu not found"
    )]
    OpenMenuTargetMissing { target_menu: Box<str> },
}

pub struct UiActionContext<'a> {
    pub known_menus: &'a BTreeSet<UiMenuId>,
    pub diagnostic_log: &'a mut dyn FnMut(&str),
}

pub trait UiActionDispatcher {
    fn dispatch(
        &mut self,
        action: &UiAction,
        context: &mut UiActionContext<'_>,
    ) -> Result<UiActionResult, UiRuntimeError>;
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
}

impl UiActionDispatcher for BuiltinUiActionDispatcher {
    fn dispatch(
        &mut self,
        action: &UiAction,
        context: &mut UiActionContext<'_>,
    ) -> Result<UiActionResult, UiRuntimeError> {
        match action {
            UiAction::OpenMenu(menu_id) => {
                if !context.known_menus.contains(menu_id) {
                    return Err(UiRuntimeError::OpenMenuTargetMissing {
                        target_menu: menu_id.to_string().into(),
                    });
                }
                self.menu_stack.push(menu_id.clone());
                Ok(UiActionResult::MenuChanged)
            }
            UiAction::BackMenu => {
                if self.menu_stack.back() {
                    Ok(UiActionResult::MenuChanged)
                } else {
                    Ok(UiActionResult::Noop)
                }
            }
            UiAction::DiagnosticLog(message) => {
                (context.diagnostic_log)(message);
                Ok(UiActionResult::Noop)
            }
        }
    }
}

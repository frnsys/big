use std::collections::BTreeMap;

use uuid::Uuid;

use crate::objects::Object;

pub type State = BTreeMap<Uuid, Object>;

pub struct Stack {
    snapshots: Vec<State>,
    position: usize,
}
impl Stack {
    pub fn new(snapshot: State) -> Self {
        Self {
            snapshots: vec![snapshot],
            position: 0,
        }
    }

    pub fn push(&mut self, snapshot: State) {
        let _ = self.snapshots.split_off(self.position + 1);
        self.snapshots.push(snapshot);
        self.position = self.snapshots.len() - 1;
    }

    pub fn undo(&mut self) -> Option<&State> {
        if self.snapshots.is_empty() {
            None
        } else {
            self.position = self.position.saturating_sub(1);
            Some(&self.snapshots[self.position])
        }
    }

    pub fn redo(&mut self) -> Option<&State> {
        if self.snapshots.is_empty() {
            None
        } else {
            self.position = (self.position + 1).min(self.snapshots.len() - 1);
            Some(&self.snapshots[self.position])
        }
    }

    pub fn current(&self) -> &State {
        &self.snapshots[self.position]
    }

    pub fn can_undo(&self) -> bool {
        self.position > 0
    }

    pub fn can_redo(&self) -> bool {
        self.position < self.snapshots.len() - 1
    }
}
